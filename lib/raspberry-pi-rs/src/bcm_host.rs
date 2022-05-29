///! A port of the `bcm_host_*` functions.
///
///! See `https://github.com/raspberrypi/firmware/blob/93420678493d9bb4ce97ab03d3fe87d60b1b3aa0/hardfp/opt/vc/include/bcm_host.h`
///! Also see `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c`
use crate::Error;
use once_cell::sync::OnceCell;
use std::{
    fs::File,
    io::{
        BufRead,
        BufReader,
    },
};

/// The board type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum BoardType {
    ModelA = 0x00,
    ModelB = 0x01,
    ModelAPlus = 0x02,
    ModelBPlus = 0x03,
    Pi2ModelB = 0x04,
    Alpha = 0x05,
    Cm = 0x06,
    Cm2 = 0x07,
    Pi3ModelB = 0x08,
    Pi0 = 0x09,
    Cm3 = 0x0A,
    Custom = 0x0B,
    Pi0W = 0x0C,
    Pi3ModelBPlus = 0x0D,
    Pi3ModelAPlus = 0x0E,
    Fpga = 0x0F,
    Cm3Plus = 0x10,
    Pi4ModelB = 0x11,
    Pi400 = 0x13,
    Cm4 = 0x14,
}

impl TryFrom<u8> for BoardType {
    type Error = u8;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            0x00 => Ok(Self::ModelA),
            0x01 => Ok(Self::ModelB),
            0x02 => Ok(Self::ModelAPlus),
            0x03 => Ok(Self::ModelBPlus),
            0x04 => Ok(Self::Pi2ModelB),
            0x05 => Ok(Self::Alpha),
            0x06 => Ok(Self::Cm),
            0x07 => Ok(Self::Cm2),
            0x08 => Ok(Self::Pi3ModelB),
            0x09 => Ok(Self::Pi0),
            0x0A => Ok(Self::Cm3),
            0x0B => Ok(Self::Custom),
            0x0C => Ok(Self::Pi0W),
            0x0D => Ok(Self::Pi3ModelBPlus),
            0x0E => Ok(Self::Pi3ModelAPlus),
            0x0F => Ok(Self::Fpga),
            0x10 => Ok(Self::Cm3Plus),
            0x11 => Ok(Self::Pi4ModelB),
            0x13 => Ok(Self::Pi400),
            0x14 => Ok(Self::Cm4),
            _ => Err(n),
        }
    }
}

/// The id of the processor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum ProcessorId {
    Bcm2835 = 0x00,
    Bcm2836 = 0x01,
    Bcm2837 = 0x02,

    /// This is also Bcm2838, which is a deprecated name for this id.
    Bcm2711 = 0x03,
}

impl TryFrom<u8> for ProcessorId {
    type Error = u8;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            0x00 => Ok(Self::Bcm2835),
            0x01 => Ok(Self::Bcm2836),
            0x02 => Ok(Self::Bcm2837),
            0x03 => Ok(Self::Bcm2711),
            _ => Err(n),
        }
    }
}

/// Get the revision code.
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L208-L215`.
fn get_revision_code() -> Result<u32, Error> {
    static REVISION_NUM: OnceCell<u32> = OnceCell::new();
    REVISION_NUM
        .get_or_try_init(|| {
            let file = BufReader::new(File::open("/proc/cpuinfo")?);
            for line in file.lines() {
                if let Some(revision) = line?.strip_prefix("Revision") {
                    let revision = revision.trim().trim_start_matches(':').trim();
                    return Ok(u32::from_str_radix(revision, 16)?);
                }
            }

            Err(Error::NotFound)
        })
        .copied()
}

/// Get the type of pi being used
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L219-L266`.
pub fn get_model_type() -> Result<BoardType, Error> {
    const TYPE_MAP: &[BoardType] = &[
        BoardType::ModelB,     // B rev 1.0  2
        BoardType::ModelB,     // B rev 1.0  3
        BoardType::ModelB,     // B rev 2.0  4
        BoardType::ModelB,     // B rev 2.0  5
        BoardType::ModelB,     // B rev 2.0  6
        BoardType::ModelA,     // A rev 2    7
        BoardType::ModelA,     // A rev 2    8
        BoardType::ModelA,     // A rev 2    9
        BoardType::ModelA,     // unused a
        BoardType::ModelA,     // unused b
        BoardType::ModelA,     // unused c
        BoardType::ModelB,     // B  rev 2.0  d
        BoardType::ModelB,     // B rev 2.0  e
        BoardType::ModelB,     // B rev 2.0  f
        BoardType::ModelBPlus, // B+ rev 1.2 10
        BoardType::Cm,         // CM1        11
        BoardType::ModelAPlus, // A+ rev1.1  12
        BoardType::ModelBPlus, // B+ rev 1.2 13
        BoardType::Cm,         // CM1        14
        BoardType::ModelAPlus, // A+         15
    ];

    static MODEL_TYPE: OnceCell<BoardType> = OnceCell::new();
    MODEL_TYPE
        .get_or_try_init(|| {
            let mut revision_num = get_revision_code()?;
            if revision_num == 0 {
                Ok(BoardType::ModelA)
            } else if revision_num & 0x800000 != 0 {
                // Check for old/new style revision code. Bit 23 will be guaranteed one for new style
                BoardType::try_from(u8::try_from((revision_num & 0xff0) >> 4).unwrap())
                    .map_err(|_n| Error::NotFound)
            } else {
                // Mask off warrantee and overclock bits.
                revision_num &= 0xffffff;

                // Map old style to new Type code
                if !(2..=21).contains(&revision_num) {
                    return Ok(BoardType::ModelA);
                }

                TYPE_MAP
                    .get(usize::try_from(revision_num - 2).map_err(|_e| Error::NotFound)?)
                    .ok_or(Error::NotFound)
                    .copied()
            }
        })
        .copied()
}

/// Test if the host is a member of the Pi 4 family (4B, 400 and CM4)
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L270-L273`
pub fn is_model_pi4() -> Result<bool, Error> {
    Ok(get_processor_id()? == ProcessorId::Bcm2711)
}

/// Get the processor id
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L277-L290`.
pub fn get_processor_id() -> Result<ProcessorId, Error> {
    let revision_num = get_revision_code()?;

    if revision_num & 0x800000 != 0 {
        ProcessorId::try_from(u8::try_from((revision_num & 0xf000) >> 12).unwrap())
            .map_err(|_n| Error::NotFound)
    } else {
        // Old style number only used 2835
        Ok(ProcessorId::Bcm2835)
    }
}

/// A version of [`std::fs::read_to_string`] which returns `None` for missing files
fn try_read_to_string<P>(path: P) -> Result<Option<String>, std::io::Error>
where
    P: AsRef<std::path::Path>,
{
    match std::fs::read_to_string(path) {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Check if fkms or kms is active.
///
/// # Changes
/// In contrast with the original function, this one provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L293-L300`.
fn is_fkms_or_kms_active(kms: bool) -> Result<bool, Error> {
    if !try_read_to_string("/proc/device-tree/soc/v3d@7ec00000/status")?
        .map_or(false, |data| data.contains("okay"))
        && !try_read_to_string("/proc/device-tree/v3dbus/v3d@7ec04000/status")?
            .map_or(false, |data| data.contains("okay"))
    {
        Ok(false)
    } else {
        Ok(
            (std::fs::read_to_string("/proc/device-tree/soc/firmwarekms@7e600000/status")?
                .contains("okay"))
                ^ kms,
        )
    }
}

/// Check if fkms is active.
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L302-L308`
pub fn is_fkms_active() -> Result<bool, Error> {
    static ACTIVE: OnceCell<bool> = OnceCell::new();
    ACTIVE
        .get_or_try_init(|| is_fkms_or_kms_active(false))
        .copied()
}

/// Check if kms is active.
///
/// # Changes
/// In contrast with the original function, this one is thread-safe and provides error info.
///
/// Ported from `https://github.com/raspberrypi/userland/blob/c4fd1b8986c6d6d4ae5cd51e65a8bbeb495dfa4e/host_applications/linux/libs/bcm_host/bcm_host.c#L310-L316`
pub fn is_kms_active() -> Result<bool, Error> {
    static ACTIVE: OnceCell<bool> = OnceCell::new();
    ACTIVE
        .get_or_try_init(|| is_fkms_or_kms_active(true))
        .copied()
}
