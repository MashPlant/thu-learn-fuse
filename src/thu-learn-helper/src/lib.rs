#![allow(non_snake_case)]
#![feature(async_closure)]

mod parse;
mod urls;
pub mod types;
#[cfg(feature = "blocking")]
pub mod blocking;

use reqwest::{Client, ClientBuilder, multipart::{Form, Part}};
use futures::future::{try_join3, try_join_all};
use crate::{parse::*, urls::*, types::*};

#[macro_use]
mod macros {
  // it cannot be a function, because the `Form` type in async api and blocking api of `reqwest` is different
  // it can be used in both submitting homework, or replying to discussion
  #[macro_export]
  macro_rules! form_file {
    ($form: expr, $file: expr) => {
      if let Some((name, data)) = $file {
        $form.part("fileupload", Part::bytes(data).file_name(name.to_owned()))
      } else { $form.text("fileupload", "undefined") }
    };
  }

  // `a` for `async`, `b` for `blocking`
  #[macro_export]
  macro_rules! check_success {
    (a, $req: expr, $msg: expr) => { if $req.send().await?.text().await?.contains("success") { Ok(()) } else { Err($msg.into()) } };
    (b, $req: expr, $msg: expr) => { if $req.send()?.text()?.contains("success") { Ok(()) } else { Err($msg.into()) } };
  }
}

pub struct LearnHelper(pub Client);

// compiler requires type annotation in async closure, so extract them here
const OK: Result<()> = Ok(());

impl LearnHelper {
  pub async fn login(username: &str, password: &str) -> Result<Self> {
    let client = ClientBuilder::new().cookie_store(true).user_agent(USER_AGENT).build()?;
    let params = [("i_user", username), ("i_pass", password), ("atOnce", "true")];
    let res = client.post(LOGIN).form(&params).send().await?.text().await?;
    let ticket_start = res.find("ticket=").ok_or("failed to login")? + 7; // 7 == "ticket=".len()
    let ticket_len = res[ticket_start..].find("\"").ok_or("failed to login")?;
    client.post(&AUTH_ROAM(&res[ticket_start..ticket_start + ticket_len])).send().await?;
    Ok(Self(client))
  }

  pub async fn logout(self) -> Result<()> {
    self.0.post(LOGOUT).send().await?;
    Ok(())
  }

  pub async fn semester_id_list(&self) -> Result<Vec<Id>> {
    let res = self.0.get(SEMESTER_LIST).send().await?.json::<Vec<Option<String>>>().await?;
    Ok(res.into_iter().filter_map(|x| x).collect())
  }

  pub async fn course_list(&self, semester: IdRef<'_>) -> Result<Vec<Course>> {
    let mut res = self.0.get(&COURSE_LIST(semester)).send().await?.json::<JsonWrapper1<Course>>().await?.resultList;
    try_join_all(res.iter_mut().map(async move |x| {
      x.time_location = self.0.get(&COURSE_TIME_LOCATION(&x.id)).send().await?.json().await?;
      OK
    })).await?;
    Ok(res)
  }

  pub async fn notification_list(&self, course: IdRef<'_>) -> Result<Vec<Notification>> {
    let mut res = self.0.get(&NOTIFICATION_LIST(course)).send().await?.json::<JsonWrapper2<JsonWrapper20<Notification>>>().await?.object.aaData;
    try_join_all(res.iter_mut().map(async move |x| {
      x.attachment_url = if x.attachment_name.is_some() {
        const MSG: &str = "invalid notification attachment format";
        let res = self.0.get(&NOTIFICATION_DETAIL(&x.id, course)).send().await?.text().await?;
        let href_end = res.find("\" class=\"ml-10\"").ok_or(MSG)?;
        let href_start = res[..href_end].rfind("a href=\"").ok_or(MSG)? + 8;
        Some(PREFIX.to_owned() + &res[href_start..href_end])
      } else { None };
      OK
    })).await?;
    Ok(res)
  }

  pub async fn file_list(&self, course: IdRef<'_>) -> Result<Vec<File>> {
    Ok(self.0.get(&FILE_LIST(course)).send().await?.json::<JsonWrapper2<Vec<File>>>().await?.object)
  }

  pub async fn homework_list(&self, course: IdRef<'_>) -> Result<Vec<Homework>> {
    let f = async move |f: fn(&str) -> String| {
      let mut res = self.0.get(&f(course)).send().await?.json::<JsonWrapper2<JsonWrapper20<Homework>>>().await?.object.aaData;
      try_join_all(res.iter_mut().map(async move |x| {
        let res = self.0.get(&x.detail_url()).send().await?.text().await?;
        x.detail = parse_homework_detail(&res).ok_or("invalid homework detail format")?;
        OK
      })).await?;
      Ok::<_, Error>(res)
    };
    let (mut res, mut h1, mut h2) = try_join3(f(HOMEWORK_LIST_ALL[0]), f(HOMEWORK_LIST_ALL[1]), f(HOMEWORK_LIST_ALL[2])).await?;
    res.reserve(h1.len() + h2.len());
    res.append(&mut h1);
    res.append(&mut h2);
    Ok(res)
  }

  // the performance loss caused by defining parameters as IdRef instead of impl Into<Cow<'static, str>> is negligible
  // however giving them type IdRef makes the api much clearer
  pub async fn submit_homework(&self, student_homework: IdRef<'_>, content: String, file: Option<(&str, Vec<u8>)>) -> Result<()> {
    let form = Form::new().text("zynr", content).text("xszyid", student_homework.to_owned()).text("isDeleted", "0");
    let form = form_file!(form, file);
    check_success!(a, self.0.post(HOMEWORK_SUBMIT).multipart(form), "failed to submit homework")
  }

  pub async fn discussion_list(&self, course: IdRef<'_>) -> Result<Vec<Discussion>> {
    Ok(self.0.get(&DISCUSSION_LIST(course)).send().await?.json::<JsonWrapper2<JsonWrapper21<_>>>().await?.object.resultsList)
  }

  pub async fn discussion_replies(&self, course: IdRef<'_>, discussion: IdRef<'_>, discussion_board: IdRef<'_>) -> Result<Vec<DiscussionReply>> {
    let res = self.0.get(&DISCUSSION_REPLIES(course, discussion, discussion_board)).send().await?.text().await?;
    parse_discussion_replies(&res).ok_or("invalid discussion replies format".into())
  }

  pub async fn reply_discussion(&self, course: IdRef<'_>, discussion: IdRef<'_>, content: String, respondent_reply: Option<IdRef<'_>>, file: Option<(&str, Vec<u8>)>) -> Result<()> {
    let form = Form::new().text("wlkcid", course.to_owned()).text("tltid", discussion.to_owned()).text("nr", content.to_owned());
    let form = form_file!(form, file);
    let form = if let Some(x) = respondent_reply { form.text("fhhid", x.to_owned()).text("_fhhid", x.to_owned()) } else { form };
    check_success!(a, self.0.post(REPLY_DISCUSSION).multipart(form), "failed to reply discussion")
  }

  pub async fn delete_discussion_reply(&self, course: IdRef<'_>, reply: IdRef<'_>) -> Result<()> {
    check_success!(a, self.0.post(&DELETE_DISCUSSION_REPLY(course, reply)), "failed to delete discussion reply")
  }
}