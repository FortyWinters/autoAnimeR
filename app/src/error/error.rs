use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct AnimeError {
    message: String,
}
impl AnimeError {
    pub fn new(mesg: String) -> Self 
    {
        Self {
            message: mesg
        }
    } 

}

impl Error for AnimeError {}

impl fmt::Display for AnimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
