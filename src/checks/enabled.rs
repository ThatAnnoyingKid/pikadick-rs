use crate::{
    PoiseContext,
    PoiseError,
};
use parking_lot::Mutex;
use serenity::{
    client::Context,
    framework::standard::{
        Args,
        Check,
        CommandGroup,
        CommandOptions,
        Reason,
        macros::check,
    },
    model::prelude::*,
};
use std::{
    collections::HashMap,
    sync::Arc,
};

type MutexGuard<'a, T> = parking_lot::lock_api::MutexGuard<'a, parking_lot::RawMutex, T>;

#[derive(Debug, Default, Clone)]
pub struct EnabledCheckData {
    /// The set of all commands as strings.
    command_name_cache: Arc<Mutex<Vec<String>>>,

    /// A way to look up commands by CommandOptions and fn addr.
    ///
    /// XXX MASSIVE HACK XXX
    /// This uses the addresses of the `names` field of [`CommandOptions`] impls in order to compare commands.
    /// This is necessary as this is all serenity gives to [`Check`] functions.
    /// The only reason this works is because the serenity macro for making commands is used as the only way to make commands,
    /// as it recreates each names array for each command uniquely.
    command_lookup: Arc<Mutex<HashMap<usize, String>>>,
}

impl EnabledCheckData {
    /// Make a new [`EnabledCheckData`].
    pub fn new() -> Self {
        EnabledCheckData {
            command_name_cache: Arc::new(Mutex::new(Vec::new())),
            command_lookup: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a group to have its commands enabled/disabled.
    pub fn add_groups(&self, groups: &[&CommandGroup]) {
        let mut names = Vec::with_capacity(4);
        let mut queue = Vec::new();
        let mut command_name_cache = self.command_name_cache.lock();
        let mut command_lookup = self.command_lookup.lock();

        for group in groups.iter() {
            command_name_cache.reserve(group.options.commands.len());
            queue.reserve(group.options.commands.len());

            queue.extend(group.options.commands.iter().map(|command| (0, command)));

            while let Some((depth, command)) = queue.pop() {
                let has_enabled_check = command
                    .options
                    .checks
                    .iter()
                    .any(|&check| checks_are_same(check, &ENABLED_CHECK));

                if !has_enabled_check {
                    continue;
                }

                names.truncate(depth);

                let command_name = command
                    .options
                    .names
                    .first()
                    .expect("command does not have a name");

                names.push(*command_name);
                let command_name = names.join("::");
                command_lookup.insert(
                    command.options.names.as_ptr() as usize,
                    command_name.clone(),
                );
                command_name_cache.push(command_name);

                queue.extend(
                    command
                        .options
                        .sub_commands
                        .iter()
                        .map(|command| (depth + 1, command)),
                );
            }
        }
    }

    pub fn get_command_name_from_options(&self, options: &CommandOptions) -> Option<String> {
        self.command_lookup
            .lock()
            .get(&(options.names.as_ptr() as usize))
            .cloned()
    }

    /// Returns a mutex guard to the list of command names.
    pub fn get_command_names(&self) -> MutexGuard<'_, Vec<String>> {
        self.command_name_cache.lock()
    }
}

/// Check if 2 [`Check`]s are the same.
///
/// This includes their function pointers, though the argument references do not necessarily have to point to the same check.
/// This is necessary as `serenity`'s `PartialEq` for [`Check`] only checks the name.
fn checks_are_same(check1: &Check, check2: &Check) -> bool {
    let is_same_partial_eq = check1 == check2;

    // HACK:
    // Use pointers as ids since checks have no unique identifiers
    let function1_addr = check1.function as usize;
    let function2_addr = check2.function as usize;
    let is_same_function_ptr = function1_addr == function2_addr;

    is_same_partial_eq && is_same_function_ptr
}

#[check]
#[name("Enabled")]
pub async fn enabled_check(
    _ctx: &Context,
    _msg: &Message,
    _args: &mut Args,
    _opts: &CommandOptions,
) -> Result<(), Reason> {
    Ok(())
}

/// Check if a command is enabled
pub async fn enabled(_ctx: PoiseContext<'_>) -> Result<bool, PoiseError> {
    Ok(true)
}
