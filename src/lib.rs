use crate::{error::QiError, header::*};
use byteorder::{BigEndian, ReadBytesExt};
use numpy::{PyArray, PyArray3, PyArrayMethods,};
use pyo3::prelude::*;
use pyo3_stub_gen::{define_stub_info_gatherer, derive::*};
use rawzip::{FileReader, RECOMMENDED_BUFFER_SIZE, ZipArchive};
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
    str::FromStr,
};

pub mod error;
pub mod header;

pub struct ZipReader {
    archive: ZipArchive<FileReader>,
    buffer: Vec<u8>,
}
const ENC_MUL: f64 = 5.09709161907256E-10;
const ENC_OFF: f64 = 2.3827347262939833E-5;
const CUR_MUL: f64 = 1.0E-8;
const CUR_OFF: f64 = 8.166409972203349E-13;

impl ZipReader {
    pub fn new(archive: ZipArchive<FileReader>, buffer: Vec<u8>) -> Self {
        Self { archive, buffer }
    }
    fn get_files<F>(&mut self, pred: F) -> Result<Vec<Vec<u8>>, QiError>
    where
        F: Fn(&[u8]) -> bool,
    {
        let mut matches = Vec::new();
        let mut entries = self.archive.entries(&mut self.buffer);

        while let Some(entry) = entries.next_entry()? {
            if pred(entry.file_path().as_ref()) {
                matches.push((entry.wayfinder(), entry.uncompressed_size_hint()));
            }
        }
        let mut store = Vec::with_capacity(matches.len() as usize);
        for (waypoint, size) in matches {
            let zip_entry = self.archive.get_entry(waypoint)?;

            let reader = zip_entry.reader();
            let inflater = flate2::read::DeflateDecoder::new(reader);

            let mut verifier = zip_entry.verifying_reader(inflater);
            let mut bytes = Vec::with_capacity(size as usize);

            verifier.read_to_end(&mut bytes)?;

            store.push(bytes);
        }

        return Ok(store);
    }
}

#[gen_stub_pyclass]
#[pyclass]
/// Main Class for reading Qi data files.
pub struct QiDataFIle {
    #[pyo3(get)]
    location: PathBuf,
    reader: ZipReader,
    header: QiHeader,
}

impl QiDataFIle {
    fn read_header(reader: &mut ZipReader) -> Result<QiHeader, QiError> {
        let data = reader
            .get_files(|b| b == b"shared-data/header.properties")?
            .into_iter()
            .next()
            .ok_or(QiError::NoFile {
                id: "shared-data header".to_string(),
            })?;
        let contents = String::from_utf8(data)
            .map_err(|e| QiError::InvalidUtf8Header { err: e.to_string() })?;

        let res = QiHeader::from_str(&contents)?;
        return Ok(res);
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl QiDataFIle {
    /// Creating a new QiDataFile, requires the location to where the file is stored.
    #[new]
    fn new(location: PathBuf) -> Result<Self, QiError> {
        let file_handle = File::open(&location)?;
        let mut buffer = vec![0u8; RECOMMENDED_BUFFER_SIZE];
        let archive = ZipArchive::from_file(file_handle, &mut buffer)?;

        let mut reader = ZipReader::new(archive, buffer);
        let header = QiDataFIle::read_header(&mut reader)?;
        //how to add header
        let result = Self {
            location,
            reader,
            header,
        };
        return Ok(result);
    }

    #[getter]
    fn header<'py>(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        node_to_py(py, &self.header.data)
    }

    fn get_channel_data<'py>(
        &mut self,
        name: &str,
        segment: u8,
        unit_type: &str,
        py: Python<'_>,
    ) -> Result<Py<PyArray3<f64>>, QiError> {
        let (offset, multiplier, data_type) = self.header.convert_to_type(name, unit_type)?;


        let path = format!("/{segment}/channels/{name}.dat");
        let data = self.reader.get_files(|p| p.ends_with(path.as_bytes()))?;
        //lcd-info.8.type=float-data

        let mut buff = vec![];
        for bytes in &data {
            let n = bytes.len() / 4;
            let mut cur = io::Cursor::new(bytes);
            for _ in 0..n {
                let raw =  match data_type {
                    "float-data" =>  cur.read_f32::<BigEndian>()? as f64,
                    _ => cur.read_i32::<BigEndian>()? as f64
                };
                let val = raw * multiplier + offset;
                buff.push(val);
            }
        }
        println!("{}", buff[1]);
        let size = data.len().isqrt();
        let points = data[0].len()/4;
        let res = PyArray::from_vec(py, buff)
            .reshape([size, size, points])?
            .unbind();
        Ok(res)
    }

    fn get_channels(&self) -> Result<Vec<&str>, QiError> {
        Ok(self
            .header
            .get_channels()?
            .into_iter()
            .map(|a| a.1)
            .collect())
    }

    fn get_channel_units(&self, channel_name: &str) -> Result<Vec<&str>, QiError> {
        Ok(self.header.get_unit_types(channel_name)?)
    }
}

#[pymodule]
fn qi_data_reader(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<QiDataFIle>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
