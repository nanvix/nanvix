// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::io::{
    self,
    Read,
    Write,
};

//==================================================================================================
// Constants
//==================================================================================================

const MAX_REQUEST_SIZE: usize = 4096;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() {
    let mut buffer: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
    let mut n: usize = 0;

    loop {
        match io::stdin().read(&mut buffer[n..]) {
            Ok(0) => break,          // End of file reached.
            Ok(nread) => n += nread, // Read some bytes.
            Err(_) => break,         // Error encountered.
        }
    }

    if n > 0 {
        io::stdout().write_all(&buffer[..n]).unwrap();
        io::stdout().flush().unwrap();
    }
}
