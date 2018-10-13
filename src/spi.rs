//! SPI mock implementations.
//!
//! This mock supports the specification and checking of expectations to allow
//! automated testing of SPI based drivers. Mismatches between expected and
//! real SPI transactions will cause runtime assertions to assist with locating
//! faults.
//!
//! ## Usage
//!
//! ```
//! extern crate embedded_hal;
//! extern crate embedded_hal_mock;
//!
//! use embedded_hal::blocking::spi::{Transfer, Write};
//! use embedded_hal_mock::spi::{Mock as SpiMock, Transaction as SpiTransaction};
//!
//! let mut spi = SpiMock::new();
//!
//! // Configure expectations
//! spi.expect(vec![
//!     SpiTransaction::write(vec![1, 2]),
//!     SpiTransaction::transfer(vec![3, 4], vec![5, 6]),
//! ]);
//!
//! // Writing
//! spi.write(&vec![1, 2]).unwrap();
//!
//! // Transferring
//! let mut buf = vec![3, 4];
//! spi.transfer(&mut buf).unwrap();
//! assert_eq!(buf, vec![5, 6]);
//!
//! // Finalise expectations
//! spi.done();
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use hal::blocking::spi;

use error::MockError;

/// SPI Transaction mode
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Write,
    Transfer,
}

/// SPI transaction type
///
/// Models an SPI write or transfer (with response)
#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    expected_mode: Mode,
    expected_data: Vec<u8>,
    response: Vec<u8>,
}

impl Transaction {
    /// Create a write transaction
    pub fn write(expected: Vec<u8>) -> Transaction {
        Transaction {
            expected_mode: Mode::Write,
            expected_data: expected,
            response: Vec::new(),
        }
    }

    /// Create a transfer transaction
    pub fn transfer(expected: Vec<u8>, response: Vec<u8>) -> Transaction {
        Transaction {
            expected_mode: Mode::Transfer,
            expected_data: expected,
            response,
        }
    }
}

/// Mock SPI implementation
///
/// This supports the specification and checking of expectations to allow
/// automated testing of SPI based drivers. Mismatches between expected and
/// real SPI transactions will cause runtime assertions to assist with locating
/// faults.
///
/// See the usage section in the module level docs for an example.
pub struct Mock {
    expected: Arc<Mutex<VecDeque<Transaction>>>,
}

impl Mock {
    /// Create a new mock SPI interface
    pub fn new() -> Self {
        Mock {
            expected: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Set expectations on the SPI interface
    ///
    /// This is a list of SPI transactions to be executed in order
    /// Note that setting this will overwrite any existing expectations
    pub fn expect(&mut self, expected: Vec<Transaction>) {
        let mut e = self.expected.lock().unwrap();
        *e = expected.into();
    }

    /// Assert that all expectations on a given Mock have been met
    pub fn done(&mut self) {
        let expected = self.expected.lock().unwrap();
        assert_eq!(expected.len(), 0);
    }
}

impl Clone for Mock {
    fn clone(&self) -> Mock {
        Mock{ expected: self.expected.clone() }
    }
}

impl spi::Write<u8> for Mock {
    type Error = MockError;

    /// spi::Write implementation for Mock
    ///
    /// This will cause an assertion if the write call does not match the next expectation
    fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        let w = self
            .expected
            .lock()
            .unwrap()
            .pop_front()
            .expect("no expectation for spi::write call");
        assert_eq!(w.expected_mode, Mode::Write, "spi::write unexpected mode");
        assert_eq!(&w.expected_data, &buffer, "spi::write data does not match expectation");
        Ok(())
    }
}

impl spi::Transfer<u8> for Mock {
    type Error = MockError;

    /// spi::Transfer implementation for Mock
    ///
    /// This writes the provided response to the buffer and will cause an assertion if the written data does not match the next expectation
    fn transfer<'w>(&mut self, buffer: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        let w = self
            .expected
            .lock()
            .unwrap()
            .pop_front()
            .expect("no expectation for spi::transfer call");
        assert_eq!(w.expected_mode, Mode::Transfer, "spi::transfer unexpected mode");
        assert_eq!(&w.expected_data, &buffer, "spi::write data does not match expectation");
        assert_eq!(buffer.len(), w.response.len(), "mismatched response length for spi::transfer");
        buffer.copy_from_slice(&w.responsea);
        Ok(buffer)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use hal::blocking::spi::{Transfer, Write};

    #[test]
    fn test_spi_mock_write() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::write(vec![10, 12])]);

        spi.write(&vec![10, 12]).unwrap();

        spi.done();
    }

    #[test]
    fn test_spi_mock_transfer() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::transfer(
            vec![10, 12],
            vec![12, 13],
        )]);

        let mut v = vec![10, 12];
        spi.transfer(&mut v).unwrap();

        assert_eq!(v, vec![12, 13]);

        spi.done();
    }

    #[test]
    fn test_spi_mock_multiple() {
        let mut spi = Mock::new();

        spi.expect(vec![
            Transaction::write(vec![1, 2]),
            Transaction::transfer(vec![3, 4], vec![5, 6]),
        ]);

        spi.write(&vec![1, 2]).unwrap();

        let mut v = vec![3, 4];
        spi.transfer(&mut v).unwrap();

        assert_eq!(v, vec![5, 6]);

        spi.done();
    }

    #[test]
    #[should_panic]
    fn test_spi_mock_write_err() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::write(vec![10, 12])]);

        spi.write(&vec![10, 12, 12]).unwrap();

        spi.done();
    }

    #[test]
    #[should_panic]
    fn test_spi_mock_transfer_err() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::transfer(
            vec![10, 12],
            vec![12, 15],
        )]);

        let mut v = vec![10, 12];
        spi.transfer(&mut v).unwrap();

        assert_eq!(v, vec![12, 13]);

        spi.done();
    }

    #[test]
    #[should_panic]
    fn test_spi_mock_transfer_response_err() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::transfer(
            vec![1, 2],
            vec![3, 4, 5],
        )]);

        let mut v = vec![10, 12];
        spi.transfer(&mut v).unwrap();

        assert_eq!(v, vec![12, 13]);

        spi.done();
    }

    #[test]
    #[should_panic]
    fn test_spi_mock_mode_err() {
        let mut spi = Mock::new();

        spi.expect(vec![Transaction::transfer(vec![10, 12], vec![])]);

        spi.write(&vec![10, 12, 12]).unwrap();

        spi.done();
    }

    #[test]
    #[should_panic]
    fn test_spi_mock_multiple_transaction_err() {
        let mut spi = Mock::new();

        spi.expect(vec![
            Transaction::write(vec![10, 12]),
            Transaction::write(vec![10, 12]),
        ]);

        spi.write(&vec![10, 12, 12]).unwrap();

        spi.done();
    }
}
