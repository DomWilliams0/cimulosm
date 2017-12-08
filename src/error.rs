use std::io;
use std::ffi;

error_chain! {

    types {
        Error, ErrorKind, ResultExt, SimResult;
    }

    foreign_links {
        Io(io::Error);
        Ffi(ffi::NulError);
    }

    errors {
            OsmRequest(reason: String) {
                display("osm request failed: {}", reason)
            }

            OsmParse(err: i32) {
                display("failed to parse osm with err code {}", err)
            }
    }
}
