use serde_json::ser::CharEscape;
use serde_json::ser::Formatter;
use std::io;

pub struct HexFormatter<T: Formatter> {
    f: T,
}

impl<T: Formatter> HexFormatter<T> {
    pub fn new(f: T) -> Self {
        Self { f }
    }
}

impl<T: Formatter> Formatter for HexFormatter<T> {
    #[inline]
    fn write_null<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_null(writer)
    }

    #[inline]
    fn write_bool<W: ?Sized>(&mut self, writer: &mut W, value: bool) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_bool(writer, value)
    }

    #[inline]
    fn write_i8<W: ?Sized>(&mut self, writer: &mut W, value: i8) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_i16<W: ?Sized>(&mut self, writer: &mut W, value: i16) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_i32<W: ?Sized>(&mut self, writer: &mut W, value: i32) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_i64<W: ?Sized>(&mut self, writer: &mut W, value: i64) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_u8<W: ?Sized>(&mut self, writer: &mut W, value: u8) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_u16<W: ?Sized>(&mut self, writer: &mut W, value: u16) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_u32<W: ?Sized>(&mut self, writer: &mut W, value: u32) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_u64<W: ?Sized>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    where
        W: io::Write,
    {
        write!(writer, "{:#04x}", value)
    }

    #[inline]
    fn write_f32<W: ?Sized>(&mut self, writer: &mut W, value: f32) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_f32(writer, value)
    }

    #[inline]
    fn write_f64<W: ?Sized>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_f64(writer, value)
    }

    #[inline]
    fn write_number_str<W: ?Sized>(&mut self, writer: &mut W, value: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_number_str(writer, value)
    }

    #[inline]
    fn begin_string<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_string(writer)
    }

    #[inline]
    fn end_string<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_string(writer)
    }

    #[inline]
    fn write_string_fragment<W: ?Sized>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_string_fragment(writer, fragment)
    }

    #[inline]
    fn write_char_escape<W: ?Sized>(
        &mut self,
        writer: &mut W,
        char_escape: CharEscape,
    ) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_char_escape(writer, char_escape)
    }

    #[inline]
    fn begin_array<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_array(writer)
    }

    #[inline]
    fn end_array<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_array(writer)
    }

    #[inline]
    fn begin_array_value<W: ?Sized>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_array_value(writer, first)
    }

    #[inline]
    fn end_array_value<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_array_value(writer)
    }

    #[inline]
    fn begin_object<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_object(writer)
    }

    #[inline]
    fn end_object<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_object(writer)
    }

    #[inline]
    fn begin_object_key<W: ?Sized>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_object_key(writer, first)
    }

    #[inline]
    fn end_object_key<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_object_key(writer)
    }

    #[inline]
    fn begin_object_value<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.begin_object_value(writer)
    }

    #[inline]
    fn end_object_value<W: ?Sized>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.end_object_value(writer)
    }

    #[inline]
    fn write_raw_fragment<W: ?Sized>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        self.f.write_raw_fragment(writer, fragment)
    }
}
