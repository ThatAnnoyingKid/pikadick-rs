use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use dashmap::DashMap;
use parking_lot::Mutex;
use polldaddy::{
    HtmlResponse,
    Quiz,
    VoteResponse,
};
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use slog::error;
use std::{
    convert::TryInto,
    fmt::Write,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use url::Url;

const VOTES_PER_JOB: u64 = 100;
const DELAY_PER_VOTE: u64 = 10;
const DELAY_PER_VOTE_DURATION: Duration = Duration::from_secs(DELAY_PER_VOTE);

#[derive(Clone, Default)]
pub struct PollDaddyClient {
    client: polldaddy::Client,
    jobs: Arc<DashMap<u32, Arc<Mutex<Job>>>>,
    user_job_map: Arc<DashMap<UserId, u32>>,
}

impl std::fmt::Debug for PollDaddyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: If/When polldaddy::Client gets a debug impl, derive and remove manual impl.
        f.debug_struct("PollDaddyClient")
            .field("jobs", &self.jobs)
            .field("user_job_map", &self.user_job_map)
            .finish()
    }
}

// I may want to expand this to not be copy in the future
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub enum JobError {
    QuotaUsed(u32),
    Occupied,
}

impl PollDaddyClient {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_job(&self, quiz: Quiz, user: UserId) -> Result<(), JobError> {
        let quiz_id = quiz.get_id();

        if let Some(entry) = self.user_job_map.get(&user) {
            return Err(JobError::QuotaUsed(*entry.value()));
        }

        if let Some(entry) = self.jobs.get(&quiz_id) {
            if !entry.value().lock().expired() {
                return Err(JobError::Occupied);
            }
        }

        self.user_job_map.insert(user, quiz_id);
        self.jobs
            .insert(quiz_id, Arc::new(Mutex::new(Job::new(quiz, user))));

        Ok(())
    }

    pub fn get_job_by_id(&self, id: u32) -> Option<Arc<Mutex<Job>>> {
        self.jobs.get(&id).map(|el| el.value().clone())
    }

    pub fn get_job_by_user(&self, id: UserId) -> Option<Arc<Mutex<Job>>> {
        let id = *self.user_job_map.get(&id)?.value();
        self.get_job_by_id(id)
    }

    pub fn remove_job_by_user(&self, id: UserId) -> Option<(u32, Arc<Mutex<Job>>)> {
        let id = self.user_job_map.remove(&id)?.1;
        let job = self.jobs.remove(&id)?.1;
        Some((id, job))
    }

    // TODO: pub async fn force_clean_blocking() {} It might be possible to leak memory, so using the tokio runtime to spawn a blocking cleanup operation is probably a good idea?
}

#[derive(Debug)]
pub struct Job {
    quiz: Quiz,
    answer_index: Option<usize>,
    start_time: Instant,
    user: UserId,
    count: u64,
    response: Option<VoteResponse>,
    in_progress: bool,
}

impl Job {
    pub fn new(quiz: Quiz, user: UserId) -> Self {
        Job {
            quiz,
            answer_index: None,
            start_time: Instant::now(),
            user,
            count: 0,
            response: None,
            in_progress: false,
        }
    }

    pub fn expired(&self) -> bool {
        self.answer_index.is_none() && self.start_time.elapsed() > Duration::from_secs(10 * 60)
    }

    pub fn get_votes_remaining(&self) -> u64 {
        VOTES_PER_JOB - self.count
    }

    pub fn get_estimated_time_remaining(&self) -> Duration {
        Duration::from_secs(self.get_votes_remaining()) * DELAY_PER_VOTE.try_into().unwrap()
    }
}

impl std::fmt::Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "**Job Info**")?;
        writeln!(f, "Votes sent: {}", self.count)?;
        let time_remaining = self.get_estimated_time_remaining();
        writeln!(
            f,
            "Estimated time remaining: {:02}:{:02}",
            time_remaining.as_secs() / 60,
            time_remaining.as_secs() % 60,
        )?;
        writeln!(f)?;
        if let Some(html) = self.response.as_ref().and_then(|r| r.html().ok()) {
            writeln!(f, "{}", &format_html_response(html)?)?;
        }
        Ok(())
    }
}

#[command]
#[description("Spam a polldaddy quiz with 100 votes")]
#[checks(Enabled)]
#[sub_commands("info")]
pub async fn polldaddy(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url: Result<Url, _> = args.parse();
    let number: Result<usize, _> = args.parse();

    match (url, number) {
        (Ok(_), Ok(_)) => {
            msg.channel_id.say(
                &ctx.http,
                "How in the world did you make a string be a number and a url at the same time?",
            ).await?;
        }
        (Ok(url), Err(_)) => {
            scan_url(ctx, msg, url).await?;
        }
        (Err(_), Ok(answer_index)) => {
            spam(ctx, msg, answer_index).await?;
        }
        (Err(_url_error), Err(_number_error)) => {
            msg.channel_id
                .say(&ctx.http, "Input was neither a number nor url")
                .await?; // TODO: Return meaningful errors for both
        }
    }

    Ok(())
}

#[command]
#[description("Get info about a polldaddy job")]
pub async fn info(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;

    let client = data_lock
        .get::<ClientDataKey>()
        .unwrap()
        .polldaddy_client
        .clone();

    drop(data_lock);

    let job = match client.get_job_by_user(msg.author.id) {
        Some(j) => j,
        None => {
            msg.channel_id
                .say(&ctx.http, "You have no running jobs.")
                .await?;
            return Ok(());
        }
    };

    msg.channel_id
        .say(&ctx.http, format!("{}", job.lock()))
        .await?;

    Ok(())
}

fn format_quiz(quiz: &Quiz) -> Result<String, std::fmt::Error> {
    let mut output = String::new();

    writeln!(&mut output)?;
    writeln!(&mut output, "Quiz")?;
    writeln!(&mut output, "Quiz Id: {}", quiz.get_id())?;
    writeln!(&mut output, "Quiz Hash: {}", quiz.get_hash())?;
    writeln!(&mut output, "Quiz Closed: {}", quiz.is_closed())?;
    writeln!(&mut output, "Quiz Referer: {}", quiz.get_referer())?;
    writeln!(&mut output, "Quiz Va: {}", quiz.get_va())?;
    writeln!(&mut output)?;
    writeln!(&mut output, "Answers: ")?;
    for (i, ans) in quiz.get_answers().iter().enumerate() {
        writeln!(
            &mut output,
            "{}) {} | Code {}",
            i + 1,
            ans.get_text(),
            ans.get_id()
        )?;
    }
    writeln!(&mut output)?;

    Ok(output)
}

fn format_html_response(html: &HtmlResponse) -> Result<String, std::fmt::Error> {
    let mut s = String::new();
    for (i, res) in html.get_answers().iter().enumerate() {
        match res {
            Ok(res) => {
                writeln!(
                    &mut s,
                    "{}) {} | {} votes | {}%",
                    i + 1,
                    res.get_text(),
                    res.get_votes(),
                    res.get_percent()
                )?;
            }
            Err(e) => {
                writeln!(&mut s, "{}) Failed to parse, got error: {:?}", i + 1, e)?;
            }
        };
    }
    writeln!(&mut s)?;
    writeln!(&mut s, "Total Votes: {} votes", html.get_total_votes())?;
    Ok(s)
}

async fn scan_url(ctx: &Context, msg: &Message, url: Url) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.polldaddy_client.clone();
    drop(data_lock);

    let quizzes = client.client.quiz_from_url(url.as_str()).await;

    let mut quizzes = match quizzes {
        Ok(q) => q,
        Err(e) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to get quizzes, got error: {:#?}", e),
                )
                .await?;
            return Ok(());
        }
    };
    if quizzes.is_empty() {
        msg.channel_id.say(&ctx.http, "No quizzes located.").await?;
        return Ok(());
    }

    let quiz = match quizzes.swap_remove(0) {
        Ok(q) => q,
        Err(e) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to parse quiz, got error: {:#?}", e),
                )
                .await?;
            return Ok(());
        }
    };

    if quiz.is_closed() {
        msg.channel_id.say(&ctx.http, "Quiz closed.").await?;
        return Ok(());
    }

    let mut quiz_display =
        format_quiz(&quiz).unwrap_or_else(|_| String::from("Failed to display quiz"));

    match client.insert_job(quiz, msg.author.id) {
        Ok(_) => quiz_display += "Use `polldaddy <option_number>` to select an answer.",
        Err(JobError::QuotaUsed(_id)) => {
            quiz_display += "You have reached your quota; wait for your jobs to finish";
        }
        Err(JobError::Occupied) => {
            quiz_display +=
                "A job is already running for this quiz for another user, so you cannot request another.";
        }
    }

    msg.channel_id.say(&ctx.http, quiz_display).await?;

    Ok(())
}

async fn spam(ctx: &Context, msg: &Message, answer_index: usize) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.polldaddy_client.clone();
    let logger = client_data.logger.clone();
    drop(data_lock);

    let job = match client.get_job_by_user(msg.author.id) {
        Some(j) => j,
        None => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "You do not have a job yet, use `polldaddy <url>` to start.",
                )
                .await?;
            return Ok(());
        }
    };

    if job.lock().in_progress {
        msg.channel_id
            .say(
                &ctx.http,
                "You have already started a job. Use `polldaddy info` to view the progress.",
            )
            .await?;
        return Ok(());
    }

    let (quiz, answer_count) = {
        let job_lock = job.lock();
        let quiz = job_lock.quiz.clone();
        let answer_count = job_lock.quiz.get_answers().len();

        (quiz, answer_count)
    };

    if answer_index < 1 || answer_index > answer_count {
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Invalid Option. Choose an option from 1 to {}",
                    answer_count
                ),
            )
            .await?;
        return Ok(());
    }

    job.lock().in_progress = true;

    msg.channel_id
        .say(
            &ctx.http,
            "Starting job. Use `polldaddy info` to view the progress.",
        )
        .await?;

    for _ in 0..VOTES_PER_JOB {
        match client.client.vote(&quiz, answer_index - 1).await {
            Ok(res) => {
                job.lock().response = Some(res);
            }
            Err(_e) => {
                // TODO: Save/report somehow
            }
        };
        job.lock().count += 1;
        tokio::time::sleep(DELAY_PER_VOTE_DURATION).await;
    }

    msg.channel_id
        .say(&ctx.http, format!("Sent {} votes", VOTES_PER_JOB))
        .await?;

    if client.remove_job_by_user(msg.author.id).is_none() {
        error!(logger, "Failed to release job by user id for polldaddy! Memory probably leaked and client is in an undefined state!");
    }

    Ok(())
}
