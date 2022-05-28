use std::os::raw::c_int;

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to load a library
    #[error("failed to load `{name}`")]
    LibraryLoad {
        /// The library name
        name: &'static str,
        #[source]
        error: libloading::Error,
    },

    /// bcm_host is not initialized
    #[error("bcm_host is not initialized")]
    BcmHostNotInitialized,

    /// A board type was unknown
    #[error("the board type `{0}` was unknown")]
    UnknownBoardType(c_int),

    /// `graphics_get_display_size` failed with an error code
    #[error("`graphics_get_display_size` failed with error code `{0}`")]
    GraphicsGetDisplaySize(i32),

    /// A processor id was unknown
    #[error("the processor id `{0}` is unknown")]
    UnknownProcessorId(c_int),
}

/// The board type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BoardType {
    ModelA,
    ModelB,
    ModelAPlus,
    ModelBPlus,
    Pi2ModelB,
    Alpha,
    Cm,
    Cm2,
    Pi3ModelB,
    Pi0,
    Cm3,
    Custom,
    Pi0W,
    Pi3ModelBPlus,
    Pi3ModelAPlus,
    Fpga,
    Cm3Plus,
    Pi4ModelB,
    Pi400,
    Cm4,
}

impl BoardType {
    /// Make a new [`BoardType`] from a [`c_int`].
    fn new(n: c_int) -> Result<Self, c_int> {
        match u32::from_ne_bytes(n.to_ne_bytes()) {
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_MODELA => Ok(Self::ModelA),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_MODELB => Ok(Self::ModelB),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_MODELAPLUS => Ok(Self::ModelAPlus),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_MODELBPLUS => Ok(Self::ModelBPlus),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI2MODELB => Ok(Self::Pi2ModelB),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_ALPHA => Ok(Self::Alpha),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CM => Ok(Self::Cm),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CM2 => Ok(Self::Cm2),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI3MODELB => Ok(Self::Pi3ModelB),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI0 => Ok(Self::Pi0),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CM3 => Ok(Self::Cm3),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CUSTOM => Ok(Self::Custom),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI0W => Ok(Self::Pi0W),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI3MODELBPLUS => {
                Ok(Self::Pi3ModelBPlus)
            }
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI3MODELAPLUS => {
                Ok(Self::Pi3ModelAPlus)
            }
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_FPGA => Ok(Self::Fpga),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CM3PLUS => Ok(Self::Cm3Plus),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI4MODELB => Ok(Self::Pi4ModelB),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_PI400 => Ok(Self::Pi400),
            raspberry_pi_sys::libbcm_host::BCM_HOST_BOARD_TYPE_CM4 => Ok(Self::Cm4),
            _ => Err(n),
        }
    }
}

/// The id of the processor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessorId {
    Bcm2835,
    Bcm2836,
    Bcm2837,

    /// This is also Bcm2838, which is a deprecated name for this id.
    Bcm2711,
}

impl ProcessorId {
    /// Try to make a new [`ProcessorId`] from a [`c_int`]
    fn new(n: c_int) -> Result<Self, c_int> {
        match u32::from_ne_bytes(n.to_ne_bytes()) {
            raspberry_pi_sys::libbcm_host::BCM_HOST_PROCESSOR_BCM2835 => Ok(Self::Bcm2835),
            raspberry_pi_sys::libbcm_host::BCM_HOST_PROCESSOR_BCM2836 => Ok(Self::Bcm2836),
            raspberry_pi_sys::libbcm_host::BCM_HOST_PROCESSOR_BCM2837 => Ok(Self::Bcm2837),
            raspberry_pi_sys::libbcm_host::BCM_HOST_PROCESSOR_BCM2838 => Ok(Self::Bcm2711),
            _ => Err(n),
        }
    }
}

/// An interface to RaspberryPi libraries
pub struct RaspberryPi {
    bcm_host: raspberry_pi_sys::libbcm_host::libbcm_host,
    bcm_host_initialized: bool,
}

impl RaspberryPi {
    /// Initialize a new [`RaspberryPi`].
    ///
    /// # Safety
    /// 1. There may only be 1 instance of this struct in the program at any given time.
    /// 2. The libraries `libbcm_host.so` in the search path must be the correct libraries with the correct function definitions.
    /// 3. No other calls may be made to the VideoCore may be made in this process by any means while this struct is active.
    /// This includes linking and using FFMpeg in-process.
    pub unsafe fn new() -> Result<Self, Error> {
        let bcm_host =
            raspberry_pi_sys::libbcm_host::libbcm_host::new("libbcm_host.so").map_err(|error| {
                Error::LibraryLoad {
                    name: "libbcm_host.so",
                    error,
                }
            })?;

        Ok(Self {
            bcm_host,
            bcm_host_initialized: false,
        })
    }

    // TODO: Call on load?
    /// Init bcm_host.
    ///
    /// This must be called before any other functions.
    ///
    /// Right now, this is safe to call multiple times, even while already initialized.
    pub fn bcm_host_init(&mut self) {
        unsafe { self.bcm_host.bcm_host_init() };
        self.bcm_host_initialized = true;
    }

    /// Deinit bcm_host.
    ///
    /// The impl is currently a no-op, but this may change in the future.
    ///
    /// # Safety
    /// The user must be done using the GPU.
    pub unsafe fn bcm_host_deinit(&mut self) -> Result<(), Error> {
        if self.bcm_host_initialized == false {
            return Err(Error::BcmHostNotInitialized);
        }
        self.bcm_host.bcm_host_deinit();
        self.bcm_host_initialized = false;

        Ok(())
    }

    /// Get the size of the graphics display.
    pub fn graphics_get_display_size(&mut self, display_number: u16) -> Result<(u32, u32), Error> {
        if self.bcm_host_initialized == false {
            return Err(Error::BcmHostNotInitialized);
        }

        let mut width = 0;
        let mut height = 0;
        let error_code = unsafe {
            self.bcm_host
                .graphics_get_display_size(display_number, &mut width, &mut height)
        };

        if error_code < 0 {
            return Err(Error::GraphicsGetDisplaySize(error_code));
        }

        Ok((width, height))
    }

    /// Get the model type
    pub fn get_model_type(&mut self) -> Result<BoardType, Error> {
        if self.bcm_host_initialized == false {
            return Err(Error::BcmHostNotInitialized);
        }
        BoardType::new(unsafe { self.bcm_host.bcm_host_get_model_type() })
            .map_err(|board_type| Error::UnknownBoardType(board_type))
    }

    /// Return `true` if this is a pi 4, or in the same family.
    pub fn is_model_pi4(&mut self) -> bool {
        unsafe { self.bcm_host.bcm_host_is_model_pi4() == 1 }
    }

    /// Return `true` if fkms is active.
    pub fn is_fkms_active(&mut self) -> bool {
        unsafe { self.bcm_host.bcm_host_is_fkms_active() == 1 }
    }

    /// Return `true` if kms is active.
    pub fn is_kms_active(&mut self) -> bool {
        unsafe { self.bcm_host.bcm_host_is_kms_active() == 1 }
    }

    /// Get the processor id.
    pub fn get_processor_id(&mut self) -> Result<ProcessorId, Error> {
        if self.bcm_host_initialized == false {
            return Err(Error::BcmHostNotInitialized);
        }
        ProcessorId::new(unsafe { self.bcm_host.bcm_host_get_processor_id() })
            .map_err(|id| Error::UnknownProcessorId(id))
    }
}
