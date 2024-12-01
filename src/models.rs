use serde::Deserialize;

// Data types to recieve and structure data

#[derive(Deserialize, Debug)]
pub struct Stock {
    pub T: String,
    pub v: f64,
    pub o: f64,
    pub c: f64,
    pub h: f64,
    pub l: f64,
    pub t: f64,
}

#[derive(Deserialize, Debug)]
pub struct StockResponse {
    pub queryCount: u64,
    pub resultsCount: u64,
    pub adjusted: bool,
    pub results: Vec<Stock>,
}
