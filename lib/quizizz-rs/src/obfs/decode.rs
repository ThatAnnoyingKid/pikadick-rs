mod options;

use crate::obfs::{
    decode::options::{
        DefaultOptions,
        Options,
        StringOptions,
    },
    BASE_CHAR_CODE_FOR_LENGTH,
    COMMON_KEY,
    MAX_CHAR_CODE,
    MIN_CHAR_CODE,
};

#[allow(clippy::absurd_extreme_comparisons)]
pub fn char_decode_default(c: char, offset: i64) -> Option<char> {
    let sum = (c as i64 + offset) as u32;

    if sum > MAX_CHAR_CODE {
        return std::char::from_u32(MIN_CHAR_CODE + (sum - MAX_CHAR_CODE) - 1);
    }

    if sum < MIN_CHAR_CODE {
        return std::char::from_u32(MAX_CHAR_CODE - (MIN_CHAR_CODE - sum) + 1);
    }

    std::char::from_u32(sum)
}

pub fn even_odd_char_decode(c: char, offset: i64, index: usize) -> Option<char> {
    if index % 2 == 0 {
        char_decode_default(c, offset)
    } else {
        char_decode_default(c, -offset)
    }
}

pub fn extract_version(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }

    s.chars().last()?.to_digit(10)
}

pub fn extract_key(ostr: &str) -> Option<String> {
    let target_char = *ostr.as_bytes().iter().nth(ostr.len() - 2)?;
    let key_len = target_char as usize - BASE_CHAR_CODE_FOR_LENGTH;

    let end_byte_pos = ostr.char_indices().nth(key_len)?.0;
    let key = &ostr.get(..end_byte_pos)?;

    deobfuscate(key, COMMON_KEY, DefaultOptions)
}

pub fn deobfuscate<O: Options>(ostr: &str, key: &str, opts: O) -> Option<String> {
    let version = extract_version(ostr).unwrap_or(0); // Key has no version, but it isn't used while decoding.
    let offset = -(opts.extract_key_sum(key, ostr)? as i64);
    let ostr = opts.modify_str(ostr, key, 0)?;

    ostr.chars()
        .enumerate()
        .map(|(i, c)| opts.decode_char(c, offset, i, version))
        .collect()
}

pub fn decode_str(ostr: &str) -> Option<String> {
    let key = extract_key(&ostr)?;
    deobfuscate(ostr, &key, StringOptions)
}

fn is_char_valid_for_obfs(c: char) -> bool {
    let c = u32::from(c);

    if c >= 0xD800 && c <= 0xDBFF {
        return false;
    }

    if c >= 0xDC00 && c <= 0xDFFF {
        return false;
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;
    const SAMPLE_1: &str = "º¹ÒçÖ¿à¶ããàãĶｧĚﾤĞﾮğﾤĚｧõﾳİﾱħｱÝﾪĭﾷĪﾷÝｿĶｧĨﾪĮﾸĜﾬĠｧõｧčﾴĪﾲÛﾳĪﾹÛﾫĪﾺĩﾩÝｱÝﾹĴﾵĠｧõｧĭﾴĪﾲéﾓĊﾙĚﾋĊﾚĉﾉÝￂĸ-2";
    const SAMPLE_2: &str = "¦Ö¥Ó¥¦¥¤Ò©Õ¤Õ¥¡¡¢Ó£ÕÓ¢Ó©èﾵÌ\u{fff2}Ð￼Ñ\u{fff2}Ìﾵ§\u{1}â\u{ffff}Ù\u{ffbf}\u{8f}\u{fffb}Î\u{6}Õﾵ§ﾵ¢\u{fff8}¡\u{fff5}¡\u{ffc8}¡ￆÎￋÑￆÑￇ\u{9d}ￃ\u{9e}\u{fff5}\u{9f}\u{fff7}ÏￄÏￋ\u{8f}\u{ffbf}\u{8f}\u{7}æ\u{3}Òﾵ§ﾵÎ\u{6}æ\u{1}Ðﾵ\u{99}ﾵÒ\u{b}Ý￼ß\u{c}\u{8f}ￍ\u{9e}ￄ\u{9f}ￆ\u{9e}\u{ffc9}¤\u{ffc1}\u{9e}ￃ\u{9f}\u{ffbf}\u{8f}\u{fff6}ß\u{fff8}Î\u{7}Ò\u{fff7}®\u{7}\u{8f}ￍ\u{9e}\u{ffc8}¥ￄ¦ￌ\u{9e}ￅ\u{a0}\u{ffc8}\u{9f}ￇ\u{9f}\u{ffbf}\u{8f}\tÒ\u{5}à￼Ü\u{1}\u{8f}ￍÈ\u{e}\u{8f}\u{7}æ\u{3}Òﾵ§ﾵºￖ¾ﾵ\u{99}ﾵã\u{fff8}ß\u{6}Ö\u{2}Ûﾵ§ￄê\u{fff0}\u{99}ﾵÜ\u{3}á￼Ü\u{1}àﾵ§\u{e}\u{8f}\u{0}Ò\u{0}Ò\u{6}Ò\u{7}\u{8f}ￍ\u{8f}\u{ffc8}Ð\u{fff6}Îￅ\u{9e}ￅ\u{9e}ￇ£ￊ¡\u{fff9}Óￃ\u{9d}ￄÑ\u{fff8}¤\u{fff4}£\u{fff8}¢ﾵ\u{99}ﾵÙ\u{2}Ô￼Û￥Ò\u{4}â￼ß\u{fff8}Ñﾵ§\u{fff9}Î\u{ffff}à\u{fff8}\u{99}ﾵà\u{7}â\u{fff7}Ò\u{1}á\u{ffdf}Ò\u{fff4}Ñ\u{fff8}ß\u{fff5}Ü\u{fff4}ß\u{fff7}\u{8f}ￍá\u{5}â\u{fff8}\u{99}ﾵá￼Ú\u{fff8}ßﾵ§\u{7}ß\u{8}Ò\u{ffbf}\u{8f}�â\u{0}Ï\u{ffff}Òﾵ§\u{7}ß\u{8}Ò\u{ffbf}\u{8f}�â\u{0}Ï\u{ffff}ÒￔÛ\u{6}ä\u{fff8}ß\u{6}\u{8f}ￍá\u{5}â\u{fff8}\u{99}ﾵÚ\u{fff8}Ú\u{fff8}àﾵ§\u{7}ß\u{8}Ò\u{ffbf}\u{8f}\u{6}Õ\u{2}äￔÛ\u{6}ä\u{fff8}ß\u{6}Ìￅ\u{8f}ￍ\u{8f}\u{fff4}Ù\nÎ\u{c}àﾵ\u{99}ﾵà\u{7}â\u{fff7}Ò\u{1}á￤â￼ç￥Ò\tÖ\u{fff8}ä\u{fff2}\u{9f}ﾵ§ﾵæ\u{fff8}àﾵ\u{99}ﾵà\u{fffb}Ü\n®\u{1}à\nÒ\u{5}àﾵ§\u{7}ß\u{8}Ò\u{ffbf}\u{8f}\u{6}á\u{8}Ñ\u{fff8}Û\u{7}¾\u{8}Ö\r¿\u{fff8}ã￼Ò\n\u{8f}ￍá\u{5}â\u{fff8}\u{99}ﾵÙ￼Ú￼áￔá\u{7}Ò\u{0}Ý\u{7}àﾵ§ￃ\u{99}ﾵà\u{7}â\u{fff7}Ò\u{1}á￠â\u{6}Ö\u{fff6}\u{8f}ￍá\u{5}â\u{fff8}\u{99}ﾵß\u{fff8}Ñ\u{fff8}Ú\u{3}á￼Ü\u{1}\u{8f}ￍ\u{8f}\u{c}Ò\u{6}\u{8f}\u{ffbf}\u{8f}\u{3}Ü\nÒ\u{5}â\u{3}àﾵ§ﾵæ\u{fff8}àﾵ\u{99}ﾵÛ￼Ð\u{fffe}Û\u{fff4}Ú\u{fff8}´\u{fff8}Û\u{fff8}ß\u{fff4}á\u{2}ßﾵ§\u{fff9}Î\u{ffff}à\u{fff8}ê\u{ffbf}\u{8f}\u{4}â￼çￜÑﾵ§ﾵ¢ￊÓ\u{ffc8}\u{9d}\u{ffc8}¦ￇÓ\u{fff7}\u{a0}ￅ¤\u{fff7}Ð\u{fff9}\u{9f}\u{ffc9}Î\u{fff4}Ó\u{ffc8}¤\u{ffc8}\u{8f}\u{ffbf}\u{8f}\u{fff8}å\u{3}Ò\u{5}Ö\u{0}Ò\u{1}áﾵ§ﾵä\u{5}Î\u{3}´\u{2}Ñ\u{fff2}Ú\u{fff4}Ö\u{1}\u{8f}\u{ffbf}\u{8f}\u{fffb}Ü\u{6}áￜÑﾵ§ﾵ¢\u{ffc9}ÓￅÑￄ¡\u{fff6}¡ￄ£ￊÐￌ¤ￇ\u{9f}ￇ\u{9e}ￆ\u{9f}\u{fff8}¤ￌ\u{8f}\u{ffbf}\u{8f}\u{fffb}Ü\u{6}á￦Ò\u{6}à￼Ü\u{1}¶\u{fff7}\u{8f}ￍÛ\u{8}Ù\u{ffff}\u{99}ﾵÎ\u{6}à￼Ô\u{1}Ú\u{fff8}Û\u{7}àﾵ§\u{fff9}Î\u{ffff}à\u{fff8}\u{99}ﾵÐ\u{5}Ò\u{fff4}á\u{fff8}´\u{5}Ü\u{8}Ýﾵ§\u{1}â\u{ffff}Ù\u{ffbf}\u{8f}\u{fffa}ß\u{2}â\u{3}¶\u{fff7}àﾵ§￮Ê\u{ffbf}\u{8f}\u{3}Ù\u{fff4}æ\u{fff8}ßﾵ§\u{e}\u{8f}￼àￔÙ\u{ffff}Ü\nÒ\u{fff7}\u{8f}ￍá\u{5}â\u{fff8}\u{99}ﾵÙ\u{2}Ô￼Û￥Ò\u{4}â￼ß\u{fff8}Ñﾵ§\u{fff9}Î\u{ffff}à\u{fff8}\u{99}ﾵÎ\u{7}á\u{fff8}Ú\u{3}á\u{6}\u{8f}ￍÈ\u{fff0}ê\u{10}92";

    #[test]
    fn extract_version_sample_1() {
        assert_eq!(extract_version(SAMPLE_1), Some(2));
    }

    #[test]
    fn extract_key_sample_1() {
        assert_eq!(extract_key(SAMPLE_1).as_deref(), Some("IHaveNoError"));
    }

    #[test]
    fn decode_sample_1() {
        assert_eq!(decode_str(SAMPLE_1).as_deref(), Some("{\"__cid__\":null,\"error\":{\"message\":\"Room not found\",\"type\":\"room.NOT_FOUND\"}}"));
    }

    #[test]
    fn extract_key_sample_2() {
        assert_eq!(
            extract_key(SAMPLE_2).as_deref(),
            Some("5e4b4543a8d3d4001b2db1b8")
        );
    }

    #[test]
    fn decode_sample_2() {
        assert_eq!(decode_str(SAMPLE_2).as_deref(), Some("{\"__cid__\":null,\"hash\":\"5e4b4543a8d3d4001b2db1b8\",\"type\":\"async\",\"expiry\":1123167.102,\"createdAt\":1581991235242,\"version\":[{\"type\":\"MCQ\",\"version\":1}],\"options\":{\"memeset\":\"5cca21214674ff001de7a6e5\",\"loginRequired\":false,\"studentLeaderboard\":true,\"timer\":true,\"jumble\":true,\"jumbleAnswers\":true,\"memes\":true,\"showAnswers_2\":\"always\",\"studentQuizReview_2\":\"yes\",\"showAnswers\":true,\"studentQuizReview\":true,\"limitAttempts\":0,\"studentMusic\":true,\"redemption\":\"yes\",\"powerups\":\"yes\",\"nicknameGenerator\":false},\"quizId\":\"57f50594fd327dcf26aaf575\",\"experiment\":\"wrapGod_main\",\"hostId\":\"56f2d14c4167c97424132e79\",\"hostSessionId\":null,\"assignments\":false,\"createGroup\":null,\"groupIds\":[],\"player\":{\"isAllowed\":true,\"loginRequired\":false,\"attempts\":[]}}"));
    }
}
