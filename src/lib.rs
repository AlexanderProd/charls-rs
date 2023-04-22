#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::error::Error;
use std::ffi::CStr;
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub type CharlsResult<T> = Result<T, Box<dyn Error>>;

#[derive(Default)]
pub struct CharLS {
    encoder: Option<*mut charls_jpegls_encoder>,
    decoder: Option<*mut charls_jpegls_decoder>,
}

impl CharLS {
    fn translate_error(&self, code: i32) -> CharlsResult<()> {
        if code != 0 {
            let message = unsafe {
                let msg = charls_get_error_message(code);
                CStr::from_ptr(msg)
            };
            let message = message.to_str().unwrap();
            return Err(message.into());
        }

        Ok(())
    }

    pub fn decode(&mut self, src: Vec<u8>, dst: &mut Vec<u8>, stride: u32) -> CharlsResult<()> {
        let decoder = match self.decoder {
            Some(decoder) => decoder,
            None => {
                self.decoder = Some(unsafe { charls_jpegls_decoder_create() });
                self.decoder.unwrap()
            }
        };

        if decoder.is_null() {
            return Err("Unable to start the codec".into());
        }

        let err = unsafe {
            charls_jpegls_decoder_set_source_buffer(decoder, src.as_ptr() as _, src.len())
        };

        self.translate_error(err)?;

        let err = unsafe { charls_jpegls_decoder_read_header(decoder) };

        self.translate_error(err)?;

        let size = unsafe {
            let mut computed_size: usize = 0;
            let size_err =
                charls_jpegls_decoder_get_destination_size(decoder, stride, &mut computed_size);

            if size_err == 0 {
                Some(computed_size)
            } else {
                None
            }
        };

        match size {
            Some(size) => {
                dst.resize(size, 0);
                let err = unsafe {
                    charls_jpegls_decoder_decode_to_buffer(
                        decoder,
                        dst.as_mut_ptr() as _,
                        size,
                        stride,
                    )
                };

                if err == 0 {
                    Ok(())
                } else {
                    let message = unsafe {
                        let msg = charls_get_error_message(err);
                        CStr::from_ptr(msg)
                    };
                    let message = message.to_str().unwrap();
                    Err(message.into())
                }
            }
            None => Err("Unable to compute decompressed size".into()),
        }
    }

    pub fn encode(
        &mut self,
        width: u32,
        height: u32,
        bits_per_sample: i32,
        component_count: i32,
        src: &mut Vec<u8>,
    ) -> CharlsResult<Vec<u8>> {
        let encoder = match self.encoder {
            Some(encoder) => encoder,
            None => {
                self.encoder = Some(unsafe { charls_jpegls_encoder_create() });
                self.encoder.unwrap()
            }
        };

        if encoder.is_null() {
            return Err("Unable to start the codec".into());
        }

        let frame_info = charls_frame_info {
            width,
            height,
            bits_per_sample,
            component_count,
        };

        let err = unsafe {
            charls_jpegls_encoder_set_frame_info(encoder, &frame_info as *const charls_frame_info)
        };

        if err != 0 {
            return Err("Unable to set the frame info".into());
        }

        let mut size = 0;
        let err =
            unsafe { charls_jpegls_encoder_get_estimated_destination_size(encoder, &mut size) };

        if err != 0 {
            return Err("Unable to estimage the destination size".into());
        }

        let mut buffer: Vec<u8> = vec![0; size];
        let err = unsafe {
            charls_jpegls_encoder_set_destination_buffer(
                encoder,
                buffer.as_mut_ptr() as *mut std::os::raw::c_void,
                size,
            )
        };

        if err != 0 {
            return Err("Unable to set the destination buffer".into());
        }

        let err = unsafe {
            charls_jpegls_encoder_encode_from_buffer(
                encoder,
                src.as_mut_ptr() as *mut std::os::raw::c_void,
                src.len(),
                0,
            )
        };

        if err != 0 {
            return Err("Unable to encode the image".into());
        }

        let err = unsafe { charls_jpegls_encoder_get_bytes_written(encoder, &mut size) };

        self.translate_error(err)?;

        buffer.truncate(size);
        Ok(buffer)
    }
}

impl Drop for CharLS {
    fn drop(&mut self) {
        if let Some(decoder) = self.decoder {
            unsafe {
                charls_jpegls_decoder_destroy(decoder);
            }
        }

        if let Some(encoder) = self.encoder {
            unsafe {
                charls_jpegls_encoder_destroy(encoder);
            }
        }
    }
}
