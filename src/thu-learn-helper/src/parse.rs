use chrono::{NaiveDateTime, format::ParseResult};
use serde::{Deserialize, Deserializer, de::Error};
use select::{document::Document, node::Node, predicate::{Predicate, Attr as A, Class as C, Name as N}};
use crate::{urls::*, types::{HomeworkDetail, DiscussionReply0, DiscussionReply}};

#[derive(Deserialize)]
pub struct JsonWrapper1<T> { pub resultList: Vec<T> }

#[derive(Deserialize)]
pub struct JsonWrapper2<T> { pub object: T }

#[derive(Deserialize)]
pub struct JsonWrapper20<T> { pub aaData: Vec<T> }

#[derive(Deserialize)]
pub struct JsonWrapper21<T> { pub resultsList: Vec<T> }

pub fn parse_homework_detail(html: &str) -> Option<HomeworkDetail> {
  let d = Document::from(html);
  let mut file_div = d.find(C("list").and(C("fujian")).and(C("clearfix")));
  fn name_url(n: Option<Node>) -> Option<(String, String)> {
    for n in n?.find(C("ftitle")) {
      let n = n.children().nth(1)?;
      let name = n.children().next()?.as_text()?.to_owned();
      let href = n.attr("href")?;
      let url_start = href.find("downloadUrl=")? + 12;
      return Some((name, PREFIX.to_owned() + &href[url_start..]));
    }
    None
  }
  Some(HomeworkDetail {
    description: d.find(C("list").and(C("calendar")).and(C("clearfix")).descendant(C("fl").and(C("right"))).descendant(C("c55"))).next()?.inner_html(),
    attachment_name_url: name_url(file_div.next()),
    submit_attachment_name_url: name_url(file_div.nth(1)),
    grade_attachment_name_url: name_url(file_div.next()),
  })
}

pub fn parse_discussion_replies(html: &str) -> Option<Vec<DiscussionReply>> {
  let d = Document::from(html);
  let mut ret = Vec::new();
  for (idx, n) in d.find(C("list").and(C("lists")).and(C("clearfix"))).enumerate() {
    let id = n.attr("id").and_then(|x| Some(x.get("item_".len()..)?.to_owned()));
    let content = n.find(C("right")).next()?;
    let content1 = if idx == 0 {
      let mut s = String::new();
      for ch in content.find(N("p")) {
        if let Some(x) = ch.children().next().and_then(|x| x.as_text()) { s += x; }
      }
      s
    } else { content.find(A("name", "p_nr")).next()?.inner_html() };
    let author = n.find(C("name")).next()?.inner_html();
    let time = n.find(C("time")).next()?.children().nth(1)?;
    let publish_time = date_time_hm(if idx == 0 { &time.children().next()?.as_text()? } else {
      &time.as_text()?.get("楼：".len()..)?
    }).ok()?;
    let mut replies = Vec::new();
    if let Some(reply) = content.find(C("huifu_cont").and(C("panel"))).next() {
      for item in reply.find(C("item")) {
        let id = item.attr("id").and_then(|x| Some(x.get("item_".len()..)?.to_owned())); // actually it must be Some(_)
        let content = item.find(A("name", "p_nr")).next()?;
        let author = content.prev()?.prev()?.children().next()?.as_text()?;
        let author = author.get(..author.len() - "：".len())?.to_owned();
        let publish_time = date_time_hm(item.find(C("time")).next()?.children().next()?.as_text()?).ok()?;
        replies.push(DiscussionReply0 { id, author, publish_time, content: content.inner_html(), replies: () });
      }
    }
    ret.push(DiscussionReply0 { id, author, publish_time, content: content1, replies })
  }
  Some(ret)
}

fn date_time_hm(s: &str) -> ParseResult<NaiveDateTime> { NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") }

pub fn date_time<'d, D>(d: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'d> {
  date_time_hm(<&str>::deserialize(d)?).map_err(Error::custom)
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
