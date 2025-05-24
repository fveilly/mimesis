use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidBoundingBox,
    NotEnoughPoints,
    TriangulationFailed,
    Custom(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Custom(s.to_string())
    }
}

impl From<earcutr::Error> for Error {
    fn from(err: earcutr::Error) -> Self {
        Error::Custom(format!("Earcut error: {:?}", err))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidBoundingBox => write!(f, "Invalid bounding box"),
            Error::NotEnoughPoints => write!(f, "Not enough points to form a polygon"),
            Error::TriangulationFailed => write!(f, "Triangulation failed"),
            Error::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}