use std::io;
use std::ffi;
use std::sync;

error_chain! {

    types {
        Error, ErrorKind, ResultExt, SimResult;
    }

    foreign_links {
        Io(io::Error);
        Ffi(ffi::NulError);
        Sync(sync::mpsc::RecvError);
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
