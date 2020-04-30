use chrono::NaiveDateTime;
use serde::Deserialize;
use derive_more::{From, Deref, DerefMut};
use std::fmt;
use crate::{parse::*, urls::*};

#[derive(Debug, From)]
pub enum Error {
  Network(reqwest::Error),
  Message(&'static str),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::Network(e) => write!(f, "network error: {}", e),
      Error::Message(m) => write!(f, "error: {}", m),
    }
  }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub type Id = String;
pub type IdRef<'a> = &'a str;

pub fn semester_desc(semester: IdRef) -> String {
  let (l, r) = semester.split_at(semester.len() - 1);
  l.to_owned() + match r {
    "1" => "fall",
    "2" => "spring",
    "3" => "summer",
    _ => panic!("invalid semester type"),
  }
}

#[derive(Debug, Deserialize)]
pub struct Course {
  #[serde(rename = "wlkcid")] pub id: Id,
  #[serde(rename = "kcm")] pub name: String,
  #[serde(rename = "ywkcm")] pub english_name: String,
  #[serde(rename = "jsm")] pub teacher_name: String,
  // `teacher_number` and `course_number` are normally string representation of an integer, but there are a few cases that they are not
  #[serde(rename = "jsh")] pub teacher_number: String,
  #[serde(rename = "kch")] pub course_number: String,
  #[serde(rename = "kxh")] pub course_index: u32,
  #[serde(skip)] pub time_location: Vec<String>,
}

impl Course {
  pub fn url(&self) -> String { COURSE_URL(&self.id) }
}

#[derive(Debug, Deserialize)]
pub struct Notification {
  #[serde(rename = "wlkcid")] pub course_id: Id,
  #[serde(rename = "ggid")] pub id: Id,
  #[serde(rename = "bt")] pub title: String,
  #[serde(rename = "ggnr", deserialize_with = "base64_string")] pub content: String,
  #[serde(rename = "sfyd", deserialize_with = "str_to_bool1")] pub read: bool,
  #[serde(rename = "sfqd", deserialize_with = "str_to_bool2")] pub important: bool,
  #[serde(rename = "fbsjStr", deserialize_with = "date_time")] pub publish_time: NaiveDateTime,
  #[serde(rename = "fbrxm")] pub publisher: String,
  #[serde(rename = "fjmc")] pub attachment_name: Option<String>,
  #[serde(skip)] pub attachment_url: Option<String>,
}

impl Notification {
  pub fn detail_url(&self) -> String { NOTIFICATION_DETAIL(&self.id, &self.course_id) }
}

#[derive(Debug, Deserialize)]
pub struct File {
  #[serde(rename = "wjid")] pub id: Id,
  #[serde(rename = "bt")] pub title: String,
  #[serde(rename = "ms")] pub description: String,
  #[serde(rename = "wjdx")] pub raw_size: u32,
  #[serde(rename = "fileSize")] pub size: String,
  #[serde(rename = "scsj", deserialize_with = "date_time")] pub upload_time: NaiveDateTime,
  #[serde(rename = "isNew", deserialize_with = "int_to_bool")] pub new: bool,
  #[serde(rename = "sfqd", deserialize_with = "int_to_bool")] pub important: bool,
  #[serde(rename = "llcs")] pub visit_count: u32,
  #[serde(rename = "xzcs")] pub download_cunt: u32,
  #[serde(rename = "wjlx")] pub file_type: String,
}

impl File {
  pub fn download_url(&self) -> String { FILE_DOWNLOAD(&self.id) }
}

#[derive(Debug, Deserialize, Deref, DerefMut)]
pub struct Homework {
  #[serde(rename = "wlkcid")] pub course_id: Id,
  #[serde(rename = "zyid")] pub id: Id,
  #[serde(rename = "xszyid")] pub student_homework_id: Id,
  #[serde(rename = "bt")] pub title: String,
  #[serde(rename = "kssjStr", deserialize_with = "date_time")] pub assign_time: NaiveDateTime,
  #[serde(rename = "jzsjStr", deserialize_with = "date_time")] pub deadline: NaiveDateTime,
  #[serde(rename = "scsjStr", deserialize_with = "option_date_time")] pub submit_time: Option<NaiveDateTime>,
  #[serde(rename = "zynrStr", deserialize_with = "nonempty_string")] pub submit_content: Option<String>,
  #[serde(rename = "cj")] pub grade: Option<f32>,
  #[serde(rename = "pysjStr", deserialize_with = "option_date_time")] pub grade_time: Option<NaiveDateTime>,
  #[serde(rename = "jsm", deserialize_with = "nonempty_string")] pub grader_name: Option<String>,
  #[serde(rename = "pynr", deserialize_with = "nonempty_string")] pub grade_content: Option<String>,
  #[serde(skip)]
  #[deref]
  #[deref_mut]
  pub detail: HomeworkDetail,
}

impl Homework {
  pub fn detail_url(&self) -> String { HOMEWORK_DETAIL(&self.course_id, &self.id, &self.student_homework_id) }

  // the page that you click "submit homework" in browser, not really used in submitting homework
  pub fn submit_page(&self) -> String { HOMEWORK_SUBMIT_PAGE(&self.course_id, &self.student_homework_id) }
}

#[derive(Debug, Default)]
pub struct HomeworkDetail {
  pub description: String,
  pub attachment_name_url: Option<(String, String)>,
  pub submit_attachment_name_url: Option<(String, String)>,
  pub grade_attachment_name_url: Option<(String, String)>,
}

#[derive(Debug, Deserialize)]
pub struct Discussion {
  #[serde(rename = "id")] pub id: Id,
  #[serde(rename = "bqid")] pub board_id: String,
  #[serde(rename = "bt")] pub title: String,
  #[serde(rename = "fbrxm")] pub publisher_name: String,
  #[serde(rename = "fbsj", deserialize_with = "date_time1")] pub publish_time: NaiveDateTime,
  #[serde(rename = "zhhfrxm", deserialize_with = "nonempty_string")] pub last_replier_name: Option<String>,
  #[serde(rename = "zhhfsj", deserialize_with = "option_date_time1")] pub last_reply_time: Option<NaiveDateTime>,
  #[serde(rename = "djs")] pub visit_count: u32,
  #[serde(rename = "hfcs")] pub reply_count: u32,
}

#[derive(Debug)]
pub struct DiscussionReply0<R> {
  // the first reply is publisher's content, and cannot be further replied, so it does not have an id
  pub id: Option<String>,
  pub author: String,
  pub publish_time: NaiveDateTime,
  pub content: String,
  // sub replies, `Vec<...>` if there is any, `()` if there is none
  pub replies: R,
}

pub type DiscussionReply = DiscussionReply0<Vec<DiscussionReply0<()>>>;