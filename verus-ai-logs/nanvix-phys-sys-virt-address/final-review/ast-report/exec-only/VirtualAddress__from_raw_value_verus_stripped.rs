    fn from_raw_value(raw_addr: usize) -> Result<Self, Error> {
        Ok(VirtualAddress::from_raw_value(raw_addr))
    }
