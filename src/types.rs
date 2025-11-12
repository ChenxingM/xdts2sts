use serde::Deserialize;

/// 关键帧结构
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame: u32,
    pub cell: u16,
}

/// 层结构
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub frames: Vec<Frame>,
}

/// 摄影表结构
#[derive(Debug, Clone)]
pub struct Timesheet {
    pub name: String,
    pub frame_count: u32,
    pub layers: Vec<Layer>,
}

// ========== JSON 解析用的结构体 ==========

#[derive(Debug, Deserialize)]
pub struct XDTSRoot {
    #[serde(rename = "timeTables")]
    pub time_tables: Vec<TimeTable>,
}

#[derive(Debug, Deserialize)]
pub struct TDTSRoot {
    #[serde(rename = "timeSheets")]
    pub time_sheets: Vec<TimeSheet>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSheet {
    pub header: Header,
    #[serde(rename = "timeTables")]
    pub time_tables: Vec<TimeTable>,
}

#[derive(Debug, Deserialize)]
pub struct Header {
    pub cut: String,
}

#[derive(Debug, Deserialize)]
pub struct TimeTable {
    pub name: String,
    pub duration: u32,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(rename = "timeTableHeaders")]
    pub time_table_headers: Vec<TimeTableHeader>,
}

#[derive(Debug, Deserialize)]
pub struct Field {
    #[serde(rename = "fieldId")]
    pub field_id: u32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Deserialize)]
pub struct TimeTableHeader {
    #[serde(rename = "fieldId")]
    pub field_id: u32,
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Track {
    #[serde(rename = "trackNo")]
    pub track_no: usize,
    pub frames: Vec<FrameData>,
}

#[derive(Debug, Deserialize)]
pub struct FrameData {
    pub frame: u32,
    pub data: Vec<DataItem>,
}

#[derive(Debug, Deserialize)]
pub struct DataItem {
    pub values: Vec<String>,
}
