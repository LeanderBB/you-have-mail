#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(deref_nullptr)]
#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/go-gpa-server.rs"));

#[test]
fn test_bindings() {
    unsafe {
        let handle = gpaServerNew();
        assert!(handle >= 0);
        assert_eq!(0, gpaServerDelete(handle));
    }
}
