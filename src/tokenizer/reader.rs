use crate::errors::TokenizerResult;

trait Reader<'r, 'in, B>
    where
        Self: 'in
{
    fn read_bytes_until(&mut self, byte: u8, buf: B, position: &mut usize)
                        -> TokenizerResult<Option<&'r [u8]>>;
}