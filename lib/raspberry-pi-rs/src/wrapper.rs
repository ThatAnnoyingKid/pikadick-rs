use crate::{
    BoardType,
    Error,
    ProcessorId,
};
use std::{
    ffi::CString,
    os::raw::c_int,
};

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
        if !self.bcm_host_initialized {
            return Err(Error::BcmHostNotInitialized);
        }
        self.bcm_host.bcm_host_deinit();
        self.bcm_host_initialized = false;

        Ok(())
    }

    /// Get the size of the graphics display.
    pub fn graphics_get_display_size(&mut self, display_number: u16) -> Result<(u32, u32), Error> {
        if !self.bcm_host_initialized {
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
        BoardType::new(unsafe { self.bcm_host.bcm_host_get_model_type() })
            .map_err(Error::UnknownBoardType)
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
        ProcessorId::new(unsafe { self.bcm_host.bcm_host_get_processor_id() })
            .map_err(Error::UnknownProcessorId)
    }

    /*
        vc_gencmd_init: unsafe extern "C" fn() -> c_int
    vc_gencmd_stop: unsafe extern "C" fn()
    */
    /// Send command to general command serivce
    pub fn vc_gencmd_send(&mut self, format: &str) -> Result<(), Error> {
        if !self.bcm_host_initialized {
            return Err(Error::BcmHostNotInitialized);
        }

        let format = CString::new(format)?;
        let error_code = unsafe { (self.bcm_host.vc_gencmd_send)(format.as_ptr()) };

        if error_code != 0 {
            return Err(Error::VcGenCmd(error_code));
        }

        Ok(())
    }

    /// get resonse from general command serivce
    pub fn vc_gencmd_read_response(&mut self) -> Result<CString, Error> {
        if !self.bcm_host_initialized {
            return Err(Error::BcmHostNotInitialized);
        }

        let capacity: usize = raspberry_pi_sys::libbcm_host::GENCMDSERVICE_MSGFIFO_SIZE
            .try_into()
            .expect("`GENCMDSERVICE_MSGFIFO_SIZE` is larger than a `usize`");

        let mut buffer = Vec::with_capacity(capacity);

        unsafe {
            let error_code = self.bcm_host.vc_gencmd_read_response(
                buffer.as_mut_ptr(),
                capacity
                    .try_into()
                    .expect("`GENCMDSERVICE_MSGFIFO_SIZE` is larger than a `u32`"),
            );

            if error_code != 0 {
                return Err(Error::VcGenCmd(error_code));
            }

            *buffer.as_mut_ptr().add(capacity - 1) = 0;
            let len = libc::strlen(buffer.as_ptr());
            buffer.set_len(len);
        }

        Ok(CString::new(buffer).expect("there should be only one nul"))
    }
    /*
    vc_gencmd_string_property: unsafe extern "C" fn(text: *mut c_char, property: *const c_char, value: *mut *mut c_char, length: *mut c_int) -> c_int
    vc_gencmd_number_property: unsafe extern "C" fn(text: *mut c_char, property: *const c_char, number: *mut c_int) -> c_int
    vc_gencmd_until: unsafe extern "C" fn(cmd: *mut c_char, property: *const c_char, value: *mut c_char, error_string: *const c_char, timeout: c_int) -> c_int
        */
}
