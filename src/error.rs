use std::io;
use reqwest;
use std::ffi;

error_chain! {

    types {
        Error, ErrorKind, ResultExt, SimResult;
    }

    foreign_links {
        Io(io::Error);
        Reqwest(reqwest::Error);
        Ffi(ffi::NulError);
    }

    errors {
            OsmRequest(status_code: i32) {
                display("osm request failed with status code {}", status_code)
            }

            OsmParse(err: i32) {
                display("failed to parse osm with err code {}", err)
            }
    }
}
