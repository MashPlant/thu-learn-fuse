use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, de::Error};
use scraper::{Html, Selector, ElementRef};
use crate::{urls::*, types::HomeworkDetail};

#[derive(Deserialize)]
pub struct JsonWrapper1<T> { pub resultList: Vec<T> }

#[derive(Deserialize)]
pub struct JsonWrapper2<T> { pub object: T }

#[derive(Deserialize)]
pub struct JsonWrapper20<T> { pub aaData: Vec<T> }

#[derive(Deserialize)]
pub struct JsonWrapper21<T> { pub resultsList: Vec<T> }

impl HomeworkDetail {
  pub(crate) fn from_html(detail: &str) -> Option<Self> {
    lazy_static::lazy_static! {
      static ref CONTENT: Selector = Selector::parse("div.list.calendar.clearfix>div.fl.right>div.c55").unwrap();
      static ref FILE_DIV: Selector = Selector::parse("div.list.fujian.clearfix").unwrap();
      static ref FTITLE: Selector = Selector::parse(".ftitle").unwrap();
    }
    let detail = Html::parse_document(&detail);
    let mut file_div = detail.select(&FILE_DIV);
    fn name_url(e: Option<ElementRef>) -> Option<(String, String)> {
      for e in e?.select(&FTITLE) {
        for n in e.children() {
          if let Some(e) = n.value().as_element().filter(|x| x.name() == "a") {
            let name = n.children().next()?.value().as_text()?.to_string();
            let href = e.attr("href")?;
            let url_start = href.find("downloadUrl=")? + 12;
            return Some((name, PREFIX.to_string() + &href[url_start..]));
          }
        }
      }
      None
    }
    Some(HomeworkDetail {
      description: detail.select(&CONTENT).next()?.html(),
      attachment_name_url: name_url(file_div.next()),
      submit_attachment_name_url: name_url(file_div.nth(1)),
      grade_attachment_name_url: name_url(file_div.next()),
    })
  }
}

pub fn date_time<'d, D>(d: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'d> {
  NaiveDateTime::parse_from_str(<&str>::deserialize(d)?, "%Y-%m-%d %H:%M").map_err(Error::custom)
}

pub fn date_time1<'d, D>(d: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'d> {
  NaiveDateTime::parse_from_str(<&str>::deserialize(d)?, "%Y-%m-%d %H:%M:%S").map_err(Error::custom)
}

// there is indeed some duplication, a better approach is to use the newtype pattern and define wrapper class for NaiveDateTime
// but that would involve more boilerplate code, and is harder to use
pub fn option_date_time<'d, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error> where D: Deserializer<'d> {
  match <Option<&str>>::deserialize(d)? {
    Some("") | None => Ok(None),
    Some(s) => NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").map_err(Error::custom).map(Some)
  }
}

pub fn option_date_time1<'d, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error> where D: Deserializer<'d> {
  match <Option<&str>>::deserialize(d)? {
    Some("") | None => Ok(None),
    Some(s) => NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map_err(Error::custom).map(Some)
  }
}

pub fn str_to_bool1<'d, D>(d: D) -> Result<bool, D::Error> where D: Deserializer<'d> {
  Ok(<&str>::deserialize(d)? == "是")
}

pub fn str_to_bool2<'d, D>(d: D) -> Result<bool, D::Error> where D: Deserializer<'d> {
  Ok(<&str>::deserialize(d)? == "1")
}

pub fn base64_string<'d, D>(d: D) -> Result<String, D::Error> where D: Deserializer<'d> {
  let s = <Option<&str>>::deserialize(d)?.unwrap_or("");
  Ok(String::from_utf8(base64::decode(s).map_err(Error::custom)?).map_err(Error::custom)?)
}

pub fn nonempty_string<'d, D>(d: D) -> Result<Option<String>, D::Error> where D: Deserializer<'d> {
  Ok(<Option<String>>::deserialize(d)?.filter(|s| !s.is_empty()))
}

pub fn int_to_bool<'d, D>(d: D) -> Result<bool, D::Error> where D: Deserializer<'d> {
  Ok(u32::deserialize(d)? != 0)
}