use std::ffi::{c_char, CStr, CString};

mod go;

pub struct Server(go::GoInt);

pub type Result<T> = std::result::Result<T, String>;

pub struct UserId(String);

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub struct AddressId(String);

impl AsRef<str> for AddressId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub struct LabelId(String);

impl AsRef<str> for LabelId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Server {
    pub fn new() -> Result<Self> {
        let handle = unsafe { go::gpaServerNew() };
        if handle < 0 {
            return Err("Failed to create server".to_string());
        }
        Ok(Self(handle))
    }

    pub fn url(&self) -> Result<String> {
        unsafe {
            let host = go::gpaServerUrl(self.0);
            if host.is_null() {
                return Err("Invalid Server Instance".to_string());
            }
            Ok(go_char_ptr_to_str(host))
        }
    }

    pub fn create_user(
        &self,
        name: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<(UserId, AddressId)> {
        unsafe {
            let cname = CString::new(name.as_ref()).expect("Failed to convert to CString");
            let cpwd = CString::new(password.as_ref()).expect("Failed to convert to CString");
            let mut out_user_id = std::ptr::null_mut();
            let mut out_addr_id = std::ptr::null_mut();
            if go::gpaCreateUser(
                self.0,
                cname.as_ptr(),
                cpwd.as_ptr(),
                &mut out_user_id,
                &mut out_addr_id,
            ) < 0
            {
                return Err("Failed to create user".to_string());
            }

            Ok((
                UserId(go_char_ptr_to_str(out_user_id)),
                AddressId(go_char_ptr_to_str(out_addr_id)),
            ))
        }
    }

    pub fn set_auth_timeout(&self, duration: std::time::Duration) -> Result<()> {
        unsafe {
            if go::gpaSetAuthLife(self.0, duration.as_secs() as i64) < 0 {
                return Err("Failed to set auth timeout".to_string());
            }

            Ok(())
        }
    }

    pub fn create_label(
        &self,
        user_id: &UserId,
        name: impl AsRef<str>,
        parent_id: Option<&LabelId>,
        label_type: i32,
    ) -> Result<LabelId> {
        unsafe {
            let cuser_id = CString::new(user_id.as_ref()).expect("Failed to convert to CString");
            let cname = CString::new(name.as_ref()).expect("Failed to convert to CString");
            let cparent_id =
                parent_id.map(|c| CString::new(c.as_ref()).expect("Failed to convert to CString"));

            let cparent_id_ptr = if let Some(parent_id) = cparent_id {
                parent_id.as_ptr()
            } else {
                std::ptr::null()
            };

            let mut out_label_id = std::ptr::null_mut();

            if go::gpaCreateLabel(
                self.0,
                cuser_id.as_ptr(),
                cname.as_ptr(),
                cparent_id_ptr,
                label_type as i64,
                &mut out_label_id,
            ) != 0
            {
                return Err("Failed to create label".to_string());
            }

            Ok(LabelId(go_char_ptr_to_str(out_label_id)))
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe {
            if go::gpaServerDelete(self.0) < 0 {
                panic!("Failed to close gpa test server")
            }
        }
    }
}

unsafe fn go_char_ptr_to_str(go_str: *mut c_char) -> String {
    let cstr = CStr::from_ptr(go_str);
    let str = cstr.to_string_lossy().to_string();
    go::CStrFree(go_str);
    str
}

#[test]
fn test_server() {
    let server = Server::new().expect("Failed to create server");
    let url = server.url().expect("Failed to get server url");
    assert!(!url.is_empty());
}
