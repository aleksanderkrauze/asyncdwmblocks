//! This module uses ffi bindings to export an API
//! that allows for interacting with an X11 protocol.

use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::os::raw::{c_char, c_int, c_ulong};

use x11_dl::error::OpenError;
use x11_dl::xlib::{Display, Xlib};

/// C's NULL *char pointer
const NULL: *const c_char = std::ptr::null::<c_char>();

/// This enum represents possible errors that may occurs
/// when connecting to a X server.
///
/// Every variant holds a String with user-friendly error
/// description message.
#[derive(Debug)]
pub enum X11ConnectionError {
    /// Opening [Xlib] failed
    XlibOpenError(String),
    /// Opening connection to a default display failed
    /// (null pointer returned)
    XOpenDisplayError(String),
}

impl fmt::Display for X11ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::XlibOpenError(msg) => msg,
            Self::XOpenDisplayError(msg) => msg,
        };

        write!(f, "{}", msg)
    }
}

impl Error for X11ConnectionError {}

impl From<OpenError> for X11ConnectionError {
    fn from(err: OpenError) -> Self {
        Self::XlibOpenError(format!("Couldn't load Xlib: {}", err.detail()))
    }
}

/// This struct represents a connection to a X11 server.
///
/// When this struct is constructed following things happen:
/// a connection to xlib is established, a connection to
/// default X Display is establiahed, default screen and window
/// are acquired. If following steps fail a [X11ConnectionError]
/// is returned. Connection to a Display is automatically ended
/// when this structure is [dropped](Drop).
///
/// Further interaction with X11 happens through methods of this
/// struct. All of the methods are safe and internally using
/// unsafe to call functions from C library.
///
/// # Example
/// ```
/// use asyncdwmblocks::x11::X11Connection;
/// # fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// {
///     let conn = X11Connection::new()?; // Connection to X Server is established
///
///     conn.set_root_name("Hello, world!");
/// } // Here conn is dropped and connection to X Server is safely ended
/// # Ok(())
/// # }
/// ```
pub struct X11Connection {
    /// Xlib containing pointers to X11 functions
    xlib: Xlib,
    /// Default display (asserted to be not null)
    display: *mut Display,
    /// Root window of above display and screen
    window: c_ulong,
}

impl X11Connection {
    /// Tries to connect to X server. Returns error on failure.
    pub fn new() -> Result<Self, X11ConnectionError> {
        let xlib = Xlib::open()?;
        let display: *mut Display = unsafe { (xlib.XOpenDisplay)(NULL) };
        if display.is_null() {
            return Err(X11ConnectionError::XOpenDisplayError(
                "Couldn't connect to X11 display.".to_string(),
            ));
        }
        let screen: c_int = unsafe { (xlib.XDefaultScreen)(display) };
        let window: c_ulong = unsafe { (xlib.XRootWindow)(display, screen) };

        Ok(Self {
            xlib,
            display,
            window,
        })
    }

    /// This method sets root window's name to given name.
    ///
    /// # Panics
    /// As name is converted to [CString] this method will panic
    /// if name contains a null byte.
    pub fn set_root_name(&self, name: &str) {
        let name = CString::new(name).unwrap();
        unsafe {
            (self.xlib.XStoreName)(self.display, self.window, name.as_ptr());
            (self.xlib.XFlush)(self.display);
        }
    }
}

/// When this struct is dropped a connection to
/// X11 Display is closed.
impl Drop for X11Connection {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}
