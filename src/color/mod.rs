use crate::color::utils::*;
use num_bigint::{BigInt, Sign};
use pyo3::exceptions::{PyIndexError, PyValueError, PyZeroDivisionError};
use pyo3::types::{PyList, PyTuple};
use pyo3::{pyclass, pymethods, Bound, FromPyObject, PyResult, Python};
use rand::Rng;
use std::collections::hash_map::DefaultHasher;
use std::f32;
use std::f32::consts::PI;
use std::hash::{Hash, Hasher};
use std::iter::zip;

mod utils;
pub mod consts;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[pyclass]
pub struct Color {
    #[pyo3(get, set)]
    pub r: u8,
    #[pyo3(get, set)]
    pub g: u8,
    #[pyo3(get, set)]
    pub b: u8,
    #[pyo3(get, set)]
    pub a: u8,
}

#[derive(FromPyObject)]
pub enum ColorAccessCode {
    #[pyo3(transparent, annotation = "int")]
    Integer(u8),
    #[pyo3(transparent, annotation = "str")]
    String(String),
}

#[pymethods]
impl Color {
    #[new]
    #[pyo3(signature = (r, g, b, a=255))]
    fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    #[staticmethod]
    pub fn from_srgb(r: u8, g: u8, b: u8) -> PyResult<Color> {
        Ok(Color { r, g, b, a: 255 })
    }

    #[staticmethod]
    pub fn from_decimal_rgba(r: f32, g: f32, b: f32, a: f32) -> PyResult<Color> {
        find_invalid_percentage_range(r, "Red")?;
        find_invalid_percentage_range(b, "Blue")?;
        find_invalid_percentage_range(g, "Green")?;
        find_invalid_percentage_range(a, "Alpha")?;
        Ok(to_whole_rgb(r, g, b, a))
    }

    #[staticmethod]
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32, transparency: f32) -> PyResult<Color> {
        find_invalid_percentage_range(c, "Cyan")?;
        find_invalid_percentage_range(m, "Magenta")?;
        find_invalid_percentage_range(y, "Yellow")?;
        find_invalid_percentage_range(k, "Key (Black)")?;
        find_invalid_percentage_range(transparency, "Transparency")?;
        Ok(Color {
            r: (255.0 * (1.0 - c) * (1.0 - k)).round() as u8,
            g: (255.0 * (1.0 - m) * (1.0 - k)).round() as u8,
            b: (255.0 * (1.0 - y) * (1.0 - k)).round() as u8,
            a: (transparency * 255.0) as u8,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (x, y, z, transparency=1.0))]
    pub fn from_xyz(x: f32, y: f32, z: f32, transparency: f32) -> PyResult<Color> {
        if x < 0.0 || x > 95.047 {
            return Err(PyValueError::new_err("X must be between 0 and 95"));
        } else if y < 0.0 || y > 100.0 {
            return Err(PyValueError::new_err("Y must be between 0.0 and 100.0"));
        } else if z < 0.0 && z > 108.883 {
            return Err(PyValueError::new_err("Z must be between 0.0 and 108.883"));
        }
        find_invalid_percentage_range(transparency, "Transparency")?;
        let x: f32 = x / 100.0;
        let y: f32 = y / 100.0;
        let z: f32 = z / 100.0;

        let mut r: f32 = x * 3.2406 + y * -1.5372 + z * -0.4986;
        let mut g: f32 = x * -0.9689 + y * 1.8758 + z * 0.0415;
        let mut b: f32 = x * 0.0557 + y * -0.2040 + z * 1.0570;

        r = if r > 0.0031308 {
            1.055 * (r.powf(0.41666667)) - 0.055
        } else {
            12.92 * r
        };
        g = if g > 0.0031308 {
            1.055 * (g.powf(0.41666667)) - 0.055
        } else {
            12.92 * g
        };
        b = if b > 0.0031308 {
            1.055 * (b.powf(0.41666667)) - 0.055
        } else {
            12.92 * b
        };

        Ok(to_whole_rgb(r, g, b, transparency))
    }

    #[staticmethod]
    pub fn from_lch(l: f32, c: f32, h: i16, transparency: f32) -> PyResult<Color> {
        if l < 0.0 || l > 100.0 {
            return Err(PyValueError::new_err("L must be between 0 and 100"));
        } else if l < 0.0 || c > 200.0 {
            return Err(PyValueError::new_err("C must be between 0 and 200"));
        }
        find_invalid_percentage_range(transparency, "Transparency")?;

        let mut h_scoped: f32 = (h as f32).rem_euclid(360.0);
        h_scoped = h_scoped * (PI / 180.0);
        let a: f32 = h_scoped.cos() * c;
        let b: f32 = h_scoped.sin() * c;
        Ok(Color::from_oklab(l, a, b, transparency))
    }

    #[staticmethod]
    #[pyo3(signature = (h, s, v, transparency=1.0))]
    pub fn from_hsv(h: i16, s: f32, v: f32, transparency: f32) -> PyResult<Color> {
        find_invalid_percentage_range(s, "Saturation")?;
        find_invalid_percentage_range(v, "Value")?;
        find_invalid_percentage_range(transparency, "Transparency")?;
        let mut adjusted_h: f32 = (h as f32).rem_euclid(360.0);
        adjusted_h = adjusted_h / 360.0;
        let floored_h: f32 = adjusted_h.floor();
        let diff: f32 = adjusted_h - floored_h;
        let a: f32 = v * (1.0 - s);
        let b: f32 = v * (1.0 - diff * s);
        let c: f32 = v * (1.0 - (1.0 - diff) * s);
        let index: usize = (floored_h % 6.0).floor() as usize;
        let r: f32 = [v, b, a, a, c, v][index];
        let g: f32 = [c, v, v, b, a, a][index];
        let b: f32 = [a, a, c, v, v, b][index];
        Ok(to_whole_rgb(r, g, b, transparency))
    }

    #[staticmethod]
    #[pyo3(signature = (h, s, l, transparency=1.0))]
    pub fn from_hsl(h: i16, s: f32, l: f32, transparency: f32) -> PyResult<Color> {
        find_invalid_percentage_range(s, "Saturation")?;
        find_invalid_percentage_range(l, "Lightness")?;
        find_invalid_percentage_range(transparency, "Transparency")?;
        let h_scoped = h.rem_euclid(360);
        let c: f32 = (1.0 - ((2.0 * l) - 1.0).abs()) * s;
        let x: f32 = c * (1.0 - ((((h_scoped as f32) / 60.0) % 2.0) - 1.0).abs());
        let m: f32 = l - (c / 2.0);
        let (r, g, b): (f32, f32, f32) = match h_scoped {
            0..60 => (c, x, 0.0),
            60..120 => (x, c, 0.0),
            120..180 => (0.0, c, x),
            180..240 => (0.0, x, c),
            240..300 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Ok(to_whole_rgb(r + m, g + m, b + m, transparency))
    }

    #[staticmethod]
    pub fn from_hex(hex_string: &str) -> PyResult<Color> {
        let mut adjusted_str: String = hex_string.to_string();
        if hex_string.starts_with("#") {
            adjusted_str = hex_string.strip_prefix("#").unwrap().to_string();
        }
        if adjusted_str.len() != 6 || adjusted_str.len() != 8 {
            return Err(PyValueError::new_err("Invalid Hex String Length"));
        }
        let r: Result<u8, String> = interpret_to_hex(&adjusted_str, 0..2);
        let g: Result<u8, String> = interpret_to_hex(&adjusted_str, 2..4);
        let b: Result<u8, String> = interpret_to_hex(&adjusted_str, 4..6);
        let mut a: Result<u8, String> = Ok(255);
        if adjusted_str.len() == 8 {
            a = interpret_to_hex(&adjusted_str, 4..6);
        }
        match (r, g, b, a) {
            (Ok(r), Ok(g), Ok(b), Ok(a)) => Ok(Color::new(r, g, b, a)),
            (Err(_), _, _, _) => Err(PyValueError::new_err(
                "Cannot Interpret The First Hexadecimal Part",
            )),
            (_, Err(_), _, _) => Err(PyValueError::new_err(
                "Cannot Interpret The Second Hexadecimal Part",
            )),
            (_, _, Err(_), _) => Err(PyValueError::new_err(
                "Cannot Interpret The Third Hexadecimal Part",
            )),
            (_, _, _, Err(_)) => Err(PyValueError::new_err(
                "Cannot Interpret The Fourth Hexadecimal Part",
            )),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (l, a, b, transparency=1.0))]
    pub fn from_oklab(l: f32, a: f32, b: f32, transparency: f32) -> Color {
        let l_new: f32 = l + (0.3963377774 * a) + (0.2158037573 * b);
        let a_new: f32 = l - (0.1055613458 * a) - (0.0638541728 * b);
        let b_new: f32 = l - (0.0894841775 * a) - (1.2914855480 * b);

        let l_cubed: f32 = l_new.powi(3);
        let a_cubed: f32 = a_new.powi(3);
        let b_cubed: f32 = b_new.powi(3);

        let r: f32 = (4.0767416621 * l_cubed) - (3.3077115913 * a_cubed) + (0.2309699292 * b_cubed);
        let g: f32 =
            (-1.2684380046 * l_cubed) + (2.6097574011 * a_cubed) - (0.3413193965 * b_cubed);
        let b: f32 =
            (-0.0041960863 * l_cubed) - (0.7034186147 * a_cubed) + (1.7076147010 * b_cubed);
        Color {
            r: r.floor() as u8,
            g: g.floor() as u8,
            b: b.floor() as u8,
            a: (transparency * 255.0).floor() as u8,
        }
    }

    #[staticmethod]
    pub fn mlerp(start: Color, end: Color, t: f32) -> PyResult<Color> {
        find_invalid_percentage_range(t, "t")?;
        let t_inverted: f32 = 1.0 - t;
        Ok(Color {
            r: ((t_inverted * start.r as f32) + t * (end.r as f32)).floor() as u8,
            g: ((t_inverted * start.g as f32) + t * (end.g as f32)).floor() as u8,
            b: ((t_inverted * start.b as f32) + t * (end.b as f32)).floor() as u8,
            a: ((t_inverted * start.a as f32) + t * (end.a as f32)).floor() as u8
        })
    }

    #[staticmethod]
    pub fn clerp(start: Color, end: Color, t: f32) -> PyResult<Color> {
        find_invalid_percentage_range(t, "t")?;
        let lch_start: (f32, f32, u16) = color_to_lch(start);
        let lch_end: (f32, f32, u16) = color_to_lch(end);
        let one_minus_t: f32 = 1.0 - t;
        let new_values: (f32, f32, f32, f32) = (
            (one_minus_t * lch_start.0) + (t * lch_end.0),
            (one_minus_t * lch_start.1) + (t * lch_end.1),
            (one_minus_t * (lch_start.2 as f32)) + (t * (lch_end.2 as f32)),
            (one_minus_t * (start.a as f32)) + (t * (end.a as f32)),
        );
        println!("{:?}", new_values);
        Ok(Color::from_lch(
            new_values.0,
            new_values.1,
            new_values.2.floor() as i16,
            new_values.3 / 255.0
        )?)
    }

    pub fn mlerp_inplace(&mut self, _python: Python, end: Color, t: f32) -> PyResult<()> {
        find_invalid_percentage_range(t, "t")?;
        let result: Color = Color::mlerp(*self, end, t)?;
        self.r = result.r;
        self.g = result.g;
        self.b = result.b;
        self.a = result.a;
        Ok(())
    }

    pub fn clerp_inplace(&mut self, _python: Python, end: Color, t: f32) -> PyResult<()> {
        let result: Color = Color::clerp(*self, end, t)?;
        self.r = result.r;
        self.g = result.g;
        self.b = result.b;
        self.a = result.a;
        Ok(())
    }

    #[pyo3(signature = (other, include_transparency=false))]
    pub fn add(&mut self, _python: Python, other: &Color, include_transparency: bool) -> Color {
        Color {
            r: ((self.r as u16) + (other.r as u16)).min(255) as u8,
            g: ((self.g as u16) + (other.g as u16)).min(255) as u8,
            b: ((self.b as u16) + (other.b as u16)).min(255) as u8,
            a: if include_transparency {
                ((self.a as u16) + (other.a as u16)).min(255) as u8
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (other, include_transparency=false))]
    pub fn sub(&mut self, _python: Python, other: &Color, include_transparency: bool) -> Color {
        Color {
            r: ((self.r as i16) - (other.r as i16)).max(0) as u8,
            g: ((self.g as i16) - (other.g as i16)).max(0) as u8,
            b: ((self.b as i16) - (other.b as i16)).max(0) as u8,
            a: if include_transparency {
                ((self.a as i16) - (other.a as i16)).max(0) as u8
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (scalar, include_transparency=false))]
    pub fn mul(&mut self, _python: Python, scalar: f32, include_transparency: bool) -> Color {
        if scalar <= 0.0 {
            return Color::new(0, 0, 0, if include_transparency { 0 } else { self.a });
        }
        Color {
            r: ((self.r as f32) * (scalar)).clamp(0.0, 255.0).floor() as u8,
            g: ((self.g as f32) * (scalar)).clamp(0.0, 255.0).floor() as u8,
            b: ((self.b as f32) * (scalar)).clamp(0.0, 255.0).floor() as u8,
            a: if include_transparency {
                ((self.a as f32) * (scalar)).clamp(0.0, 255.0).floor() as u8
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (scalar, include_transparency=false))]
    pub fn div(
        &mut self,
        _python: Python,
        scalar: f32,
        include_transparency: bool,
    ) -> PyResult<Color> {
        if scalar == 0.0 {
            return Err(PyZeroDivisionError::new_err("Scalar division by zero"));
        }
        Ok(Color {
            r: ((self.r as f32) / (scalar)).clamp(0.0, 255.0).floor() as u8,
            g: ((self.g as f32) / (scalar)).clamp(0.0, 255.0).floor() as u8,
            b: ((self.b as f32) / (scalar)).clamp(0.0, 255.0).floor() as u8,
            a: if include_transparency {
                ((self.a as f32) / (scalar)).clamp(0.0, 255.0).floor() as u8
            } else {
                self.a
            },
        })
    }

    #[pyo3(signature = (other, include_transparency=false))]
    pub fn tensor(&mut self, _python: Python, other: Color, include_transparency: bool) -> Color {
        Color {
            r: ((self.r as u16) * (other.r as u16)).clamp(0, 255) as u8,
            g: ((self.g as u16) * (other.g as u16)).clamp(0, 255) as u8,
            b: ((self.b as u16) * (other.b as u16)).clamp(0, 255) as u8,
            a: if include_transparency {
                ((self.a as u16) * (other.a as u16)).clamp(0, 255) as u8
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (base, include_transparency=false))]
    pub fn base_sqrt(
        &mut self,
        _python: Python,
        base: u8,
        include_transparency: bool,
    ) -> PyResult<Color> {
        if base <= 1 {
            return Err(PyValueError::new_err("Square root base has to be above 1"));
        }
        let float_base: f32 = 1.0 / (base as f32);
        let a = if include_transparency {
            (self.a as f32).powf(float_base).clamp(0.0, 255.0).floor() as u8
        } else {
            self.a
        };
        Ok(Color {
            r: (self.r as f32).powf(float_base).clamp(0.0, 255.0).floor() as u8,
            g: (self.g as f32).powf(float_base).clamp(0.0, 255.0).floor() as u8,
            b: (self.b as f32).powf(float_base).clamp(0.0, 255.0).floor() as u8,
            a,
        })
    }

    #[pyo3(signature = (other, include_transparency=false))]
    pub fn max(&self, _python: Python, other: Color, include_transparency: bool) -> Color {
        Color {
            r: self.r.max(other.r),
            g: self.g.max(other.g),
            b: self.b.max(other.b),
            a: if include_transparency {
                self.a.max(other.a)
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (other, include_transparency=false))]
    pub fn min(&self, other: Color, include_transparency: bool) -> Color {
        Color {
            r: self.r.min(other.r),
            g: self.g.min(other.g),
            b: self.b.min(other.b),
            a: if include_transparency {
                self.a.min(other.a)
            } else {
                self.a
            },
        }
    }

    #[pyo3(signature = (include_transparency=false))]
    pub fn inverse(&self, _python: Python, include_transparency: bool) -> Color {
        Color {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
            a: if include_transparency {
                255 - self.a
            } else {
                self.a
            },
        }
    }

    pub fn grayscale(&self, _python: Python) -> Color {
        let value =
            (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32).round() as u8;
        Color {
            r: value,
            g: value,
            b: value,
            a: self.a,
        }
    }

    pub fn triadic_colors<'a>(&self, python: Python<'a>) -> [Color; 2] {
        let results: (u16, f32, f32, f32) = self.to_hsl(python);
        let hue_one: i16 = ((results.0 + 120).rem_euclid(360)) as i16;
        let hue_two: i16 = ((results.0 as i16) - 120).rem_euclid(360);
        [
            Color::from_hsl(hue_one, results.1, results.2, results.3).unwrap(),
            Color::from_hsl(hue_two, results.1, results.2, results.3).unwrap()
        ]
    }

    pub fn adjust_temperature(&mut self, _python: Python, temperature: BigInt) {
        if temperature == BigInt::ZERO {
            return;
        }
        let adjusted_temp = wrap_around_bigint(if temperature > BigInt::new(Sign::Plus, vec![255]) {
            BigInt::new(Sign::Plus, vec![255])
        } else if temperature < BigInt::new(Sign::Minus, vec![255]) {
            BigInt::new(Sign::Minus, vec![255])
        } else {temperature}).1 as u16;

        self.r = ((self.r as u16) + adjusted_temp).clamp(0, 255) as u8;
        self.b = ((self.b as u16) - adjusted_temp).clamp(0, 255) as u8;
    }

    pub fn contrast(&mut self, _python: Python, factor: f32) {
        if factor == 0.0 {
            return;
        }
        let new_factor = factor + 1.0;
        self.r = (127.5 + ((self.r as f32) - 127.5) * new_factor).clamp(0.0, 255.0).floor() as u8;
        self.g = (127.5 + ((self.g as f32) - 127.5) * new_factor).clamp(0.0, 255.0).floor() as u8;
        self.b = (127.5 + ((self.b as f32) - 127.5) * new_factor).clamp(0.0, 255.0).floor() as u8;
    }

    pub fn brightness(&self, _python: Python, factor: f32) -> Color {
        if factor == 0.0 {
            return *self;
        }
        let mut adjusted_factor: f32 = factor + 1.0;
        if factor < 0.0 {
            adjusted_factor = 1.0 / (factor.abs() + 1.0);
        }
        Color {
            r: ((self.r as f32) * (adjusted_factor)).floor() as u8,
            g: ((self.g as f32) * (adjusted_factor)).floor() as u8,
            b: ((self.b as f32) * (adjusted_factor)).floor() as u8,
            a: self.a,
        }
    }

    pub fn tint(&self, python: Python, degrees: BigInt) -> PyResult<Color> {
        let new_degrees: BigInt = &degrees % BigInt::from(360);
        if new_degrees == BigInt::ZERO {
            return Ok(*self);
        }
        let hsl: (u16, f32, f32, f32) = self.to_hsl(python);
        let adjusted_degrees: i16 = wrap_around_bigint_as_i16(degrees.clone());
        let hue: i16 = ((hsl.0 as i16) + adjusted_degrees).rem_euclid(360);
        Ok(Color::from_hsl(hue, hsl.1, hsl.2, (self.a as f32) / 255.0)?)
    }

    pub fn saturate(&self, _python: Python, factor: f32) -> Color {
        if factor == 0.0 {
            return *self;
        }
        let mut hsv: (u16, f32, f32) = color_to_hsv(*self);
        hsv.1 = hsv.1 * (factor + 1.0);
        Color::from_hsv(hsv.0 as i16, hsv.1, hsv.2, (self.a as f32) / 255.0).unwrap()
    }

    #[pyo3(signature = (start=[Some(0), Some(0), Some(0), Some(0)], end=[Some(255), Some(255), Some(255), Some(255)]))]
    pub fn randomise(
        &self,
        _python: Python,
        start: [Option<u8>; 4],
        end: [Option<u8>; 4],
    ) -> PyResult<Color> {
        let mut randomized_values: [u8; 4] = [0, 0, 0, 0];
        let rgba_list: [u8; 4] = [self.r, self.g, self.b, self.a];
        let iter = zip(start.iter(), end.iter()).enumerate();
        for (index, (i, j)) in iter {
            match (i, j) {
                (Some(val1), Some(val2)) => {
                    if i >= j {
                        return Err(PyIndexError::new_err(format!(
                            "Starting & Ending Bounds Are Out Of Range For Index {}",
                            index
                        )));
                    }
                    randomized_values[index] = rand::thread_rng().gen_range(*val1..*val2);
                }
                (None, None) => randomized_values[index] = rgba_list[index],
                _ => {
                    return Err(PyValueError::new_err(
                        "Cannot have None & a integer fields on start & end at the same time",
                    ));
                }
            }
        }
        Ok(Color {
            r: randomized_values[0],
            g: randomized_values[1],
            b: randomized_values[2],
            a: randomized_values[3],
        })
    }

    pub fn get_luminance(&self, python: Python) -> f32 {
        let mut rgb: (f32, f32, f32) = self.to_decimal_rgb(python);
        rgb.0 = if rgb.0 <= 0.03928 {
            rgb.0 / 12.92
        } else {
            ((rgb.0 + 0.055) / 1.055).powf(2.4)
        };
        rgb.1 = if rgb.1 <= 0.03928 {
            rgb.1 / 12.92
        } else {
            ((rgb.1 + 0.055) / 1.055).powf(2.4)
        };
        rgb.2 = if rgb.2 <= 0.03928 {
            rgb.2 / 12.92
        } else {
            ((rgb.2 + 0.055) / 1.055).powf(2.4)
        };
        0.2126 * rgb.0 + 0.7152 * rgb.1 + 0.0722 * rgb.2
    }

    pub fn get_saturation(&self, _python: Python) -> f32 {
        let rgb_max: f32 = self.r.max(self.g).max(self.b) as f32;
        if rgb_max == 0.0 {
            return 0.0;
        }
        let rgb_min: f32 = self.r.min(self.g).min(self.b) as f32;

        (rgb_max - rgb_min) / rgb_max
    }

    #[pyo3(signature = (other, diff, include_transparency=false))]
    pub fn approx_equal(
        &self,
        _python: Python,
        other: Color,
        diff: u8,
        include_transparency: bool,
    ) -> bool {
        fn approx_equal_field(value: i16, value2: i16, diff: i16) -> bool {
            value - diff <= value2 && value2 <= value + diff
        }

        let diff_adjusted: i16 = diff as i16;
        let alpha_part = if include_transparency {
            approx_equal_field(self.a as i16, other.a as i16, diff_adjusted)
        } else {
            true
        };
        approx_equal_field(self.r as i16, other.r as i16, diff_adjusted)
            && approx_equal_field(self.r as i16, other.r as i16, diff_adjusted)
            && approx_equal_field(self.r as i16, other.r as i16, diff_adjusted)
            && alpha_part
    }

    pub fn copy(&self, _python: Python) -> Color {
        Color {r: self.r, g: self.g, b: self.b, a: self.a}
    }

    #[pyo3(signature = (include_transparency=false))]
    pub fn to_hex(&self, _python: Python, include_transparency: bool) -> String {
        let hex_str = format!("#{:x?}{:x?}{:x?}", self.r, self.g, self.b);
        if include_transparency {
            hex_str + &format!("{:x?}", self.a)
        } else {
            hex_str
        }
    }

    pub fn to_hsv(&self, _python: Python) -> (u16, f32, f32, f32) {
        let hsv: (u16, f32, f32) = color_to_hsv(*self);
        (hsv.0, hsv.1, hsv.2, (self.a as f32) / 255.0)
    }

    pub fn to_hsl(&self, _python: Python) -> (u16, f32, f32, f32) {
        let values: (u16, f32, f32, f32) = calculate_hs(*self);
        let l: f32 = (values.2 + values.3) / 2.0;
        let delta: f32 = values.2 - values.3;
        let s: f32;
        if delta == 0.0 {
            s = 0.0;
        } else {
            s = delta / (1.0 - (2.0 * l - 1.0).abs());
        }
        (values.0, s, l, (self.a as f32) / 255.0)
    }

    pub fn to_decimal_rgb(&self, _python: Python) -> (f32, f32, f32) {
        color_to_decimal_rgb(*self)
    }

    pub fn to_decimal_rgba(&self, _python: Python) -> (f32, f32, f32, f32) {
        let rgb: (f32, f32, f32) = color_to_decimal_rgb(*self);
        (rgb.0, rgb.1, rgb.2, (self.a as f32) / 255.0)
    }

    pub fn to_cmyk(&self, _python: Python) -> (f32, f32, f32, f32, f32) {
        let rgb: (f32, f32, f32) = color_to_decimal_rgb(*self);
        let k: f32 = 1.0 - rgb.0.max(rgb.1).max(rgb.2);
        let k_invert: f32 = 1.0 - k;
        let c: f32 = (k_invert - rgb.0) / k_invert;
        let m: f32 = (k_invert - rgb.1) / k_invert;
        let y: f32 = (k_invert - rgb.2) / k_invert;
        (c, m, y, k, (self.a as f32) / 255.0)
    }

    pub fn to_xyz(&self, _python: Python) -> (f32, f32, f32, f32) {
        let mut rgb: (f32, f32, f32) = color_to_decimal_rgb(*self);

        rgb.0 = if rgb.0 > 0.04045 {
            ((rgb.0 + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.0 / 12.92
        };
        rgb.1 = if rgb.1 > 0.04045 {
            ((rgb.1 + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.1 / 12.92
        };
        rgb.2 = if rgb.2 > 0.04045 {
            ((rgb.2 + 0.055) / 1.055).powf(2.4)
        } else {
            rgb.2 / 12.92
        };

        rgb.0 *= 100.0;
        rgb.1 *= 100.0;
        rgb.2 *= 100.0;

        (
            rgb.0 * 0.4124 + rgb.1 * 0.3576 + rgb.2 * 0.1805,
            rgb.0 * 0.2126 + rgb.1 * 0.7152 + rgb.2 * 0.0722,
            rgb.0 * 0.0193 + rgb.1 * 0.1192 + rgb.2 * 0.9505,
            (self.a as f32) / 255.0,
        )
    }

    pub fn to_oklab(&self, _python: Python) -> (f32, f32, f32, f32) {
        let oklab: (f32, f32, f32) = color_to_oklab(*self);
        (oklab.0, oklab.1, oklab.2, (self.a as f32) / 255.0)
    }

    pub fn to_lch(&self, _python: Python) -> (f32, f32, u16, f32) {
        let lch: (f32, f32, u16) = color_to_lch(*self);
        (lch.0, lch.1, lch.2, (self.a as f32) / 255.0)
    }

    pub fn to_rgba_list<'a>(&self, python: Python<'a>) -> Bound<'a, PyList> {
        PyList::new_bound(python, vec![self.r, self.g, self.b, self.a])
    }

    pub fn to_decimal_rgba_list<'a>(&self, python: Python<'a>) -> Bound<'a, PyList> {
        PyList::new_bound(
            python,
            vec![
                (self.r as f32) / 255.0,
                (self.g as f32) / 255.0,
                (self.b as f32) / 255.0,
                (self.a as f32) / 255.0,
            ],
        )
    }

    pub fn to_rgba_tuple<'a>(&self, python: Python<'a>) -> Bound<'a, PyTuple> {
        PyTuple::new_bound(python, vec![self.r, self.g, self.b, self.a])
    }

    pub fn __str__(&self, _python: Python) -> String {
        format!("({} : {} : {} : {})", self.r, self.g, self.b, self.a)
    }

    pub fn __repr__(&self, _python: Python) -> String {
        format!("Color({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }

    pub fn __add__(&mut self, python: Python, other: Color) -> Color {
        self.add(python, &other, true)
    }

    pub fn __sub__(&mut self, python: Python, other: Color) -> Color {
        self.sub(python, &other, true)
    }

    pub fn __mul__(&mut self, python: Python, other: f32) -> Color {
        self.mul(python, other, true)
    }

    pub fn __truediv__(&mut self, python: Python, other: f32) -> PyResult<Color> {
        self.div(python, other, true)
    }

    pub fn __floordiv__(&mut self, python: Python, other: BigInt) -> PyResult<Color> {
        self.div(python, wrap_around_bigint_as_i16(other) as f32, true)
    }

    pub fn __hash__(&self, _python: Python) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.r.hash(&mut hasher);
        self.g.hash(&mut hasher);
        self.b.hash(&mut hasher);
        self.a.hash(&mut hasher);
        hasher.finish()
    }

    pub fn __nonzero__(&self, python: Python) -> bool {
        self.__bool__(python)
    }

    pub fn __bool__(&self, _python: Python) -> bool {
        (self.r as u32) + (self.g as u32) + (self.b as u32) + (self.a as u32) != 0
    }

    pub fn __eq__(&self, _python: Python, other: Color) -> bool {
        self.r == other.r && self.b == other.b && self.g == other.g && self.a == other.a
    }
    pub fn __ne__(&self, _python: Python, other: Color) -> bool {
        self.r != other.r || self.b != other.b || self.g != other.g || self.a != other.a
    }

    pub fn __neg__(&self, python: Python) -> Color {
        self.inverse(python, true)
    }

    pub fn __pow__(&self, _python: Python, color: Color, base: f32) -> Color {
        Color {
            r: (color.r as f32).powf(base).floor() as u8,
            g: (color.g as f32).powf(base).floor() as u8,
            b: (color.b as f32).powf(base).floor() as u8,
            a: (color.a as f32).powf(base).floor() as u8,
        }
    }

    pub fn __rpow__(&self, python: Python, color: Color, base: f32) -> Color {
        self.__pow__(python, color, base)
    }

    pub fn __getitem__(&self, _python: Python, access_code: ColorAccessCode) -> PyResult<u8> {
        let adjusted_access_code = access_code;
        if let ColorAccessCode::String(value) = adjusted_access_code {
            return match value.to_lowercase().as_str() {
                "red" | "r" => Ok(self.r),
                "green" | "g" => Ok(self.g),
                "blue" | "b" => Ok(self.b),
                "alpha" | "a" => Ok(self.a),
                _ => Err(PyIndexError::new_err(
                    "Cannot access a value outside of the color's reach",
                )),
            };
        }
        match adjusted_access_code {
            ColorAccessCode::Integer(0) => Ok(self.r),
            ColorAccessCode::Integer(1) => Ok(self.g),
            ColorAccessCode::Integer(2) => Ok(self.b),
            ColorAccessCode::Integer(3) => Ok(self.a),
            _ => Err(PyIndexError::new_err(
                "Cannot access a value outside of the color's reach",
            )),
        }
    }

    pub fn __setitem__(
        &mut self,
        _python: Python,
        access_code: ColorAccessCode,
        new_value: u8,
    ) -> PyResult<()> {
        let adjusted_access_code = access_code;
        if let ColorAccessCode::String(value) = adjusted_access_code {
            return match value.to_lowercase().as_str() {
                "red" | "r" => {
                    self.r = new_value;
                    Ok(())
                }
                "green" | "g" => {
                    self.g = new_value;
                    Ok(())
                }
                "blue" | "b" => {
                    self.b = new_value;
                    Ok(())
                }
                "alpha" | "a" => {
                    self.a = new_value;
                    Ok(())
                }
                _ => Err(PyIndexError::new_err(
                    "Cannot set a value outside of the color's reach",
                )),
            };
        }
        match adjusted_access_code {
            ColorAccessCode::Integer(0) => {
                self.r = new_value;
                Ok(())
            }
            ColorAccessCode::Integer(1) => {
                self.g = new_value;
                Ok(())
            }
            ColorAccessCode::Integer(2) => {
                self.b = new_value;
                Ok(())
            }
            ColorAccessCode::Integer(3) => {
                self.a = new_value;
                Ok(())
            }
            _ => Err(PyIndexError::new_err(
                "Cannot set a value outside of the color's reach",
            )),
        }
    }

    fn shift(&self, _python: Python, places: BigInt) -> Color {
        let four: BigInt = BigInt::from(4);
        if (&places % &four) == BigInt::ZERO {
            return *self;
        }
        let mut arr: [u8; 4] = [self.r, self.g, self.b, self.a];
        let arr_clone: [u8; 4] = arr.clone();
        let mut adjusted_places: BigInt = places % &four;
        adjusted_places = if adjusted_places < BigInt::ZERO {
            &four - adjusted_places
        } else {
            adjusted_places
        };
        for (index, entry) in arr_clone.iter().enumerate() {
            let calc_index: usize = wrap_around_bigint(
                (&adjusted_places + (BigInt::from(index))) % &four,
            ).1 as usize;
            arr[calc_index] = *entry
        }
        Color {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }

    pub fn __rshift__(&self, python: Python, places: BigInt) -> Color {
        self.shift(python, places)
    }

    pub fn __lshift__(&self, python: Python, places: BigInt) -> Color {
        self.shift(python, -places)
    }

    pub fn __copy__(&self, python: Python) -> Color {
        self.copy(python)
    }

    pub fn __sizeof__(&self, _python: Python) -> usize { 32 }
}