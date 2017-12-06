use std::io;

error_chain! {

    types {
        Error, ErrorKind, ResultExt, SimResult;
    }

    foreign_links {
        Io(io::Error);
    }

    errors {
//        Test {
//            display("nothing here")
//        }
//
//        TestArgs(arg: &'static str) {
//            display("arg: {}", arg)
//        }
    }
}
