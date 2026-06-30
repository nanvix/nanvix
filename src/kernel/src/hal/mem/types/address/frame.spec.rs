verus! {

/// The architectural page size.
pub open spec fn spec_page_size() -> int {
    ::arch::mem::PAGE_SIZE as int
}

/// The integer index (raw value) of a frame number: its `arch` view.
pub open spec fn spec_frame_raw_value(frame: FrameNumber) -> int {
    frame@
}

/// The largest representable frame index (the `arch` bound `FrameNumber::spec_max()`).
pub open spec fn spec_max_frame_number() -> int {
    FrameNumber::spec_max() as int
}

/// The frame an address belongs to: `addr / FRAME_SIZE` (equivalently `addr >> FRAME_SHIFT`).
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}

/// The base address of a frame: `frame_index * FRAME_SIZE`.
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}

impl View for FrameAddress
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.0@
    }
}

impl FrameAddress {
    pub open spec fn inv(&self) -> bool
    {
        self@ % spec_page_size() == 0
    }
}

}
