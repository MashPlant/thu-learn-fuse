use reqwest::{blocking::{Client, ClientBuilder, multipart::{Form, Part}}};
use std::borrow::Cow;
use crate::{parse::*, urls::*, types::*};

// the inner `Client` object is public, because I don't pretty much care user modifying it
// after all they will have to pay a price (getting `Err` result) if their modification is not proper
pub struct LearnHelper(pub Client);

impl LearnHelper {
  pub fn login(username: &str, password: &str) -> Result<Self> {
    let client = ClientBuilder::new().cookie_store(true).user_agent(USER_AGENT).build()?;
    let params = [("i_user", username), ("i_pass", password), ("atOnce", "true")];
    let res = client.post(LOGIN).form(&params).send()?.text()?;
    let ticket_start = res.find("ticket=").ok_or("failed to login")? + 7; // 7 == "ticket=".len()
    let ticket_len = res[ticket_start..].find("\"").ok_or("failed to login")?;
    client.post(&AUTH_ROAM(&res[ticket_start..ticket_start + ticket_len])).send()?;
    Ok(Self(client))
  }

  // you may logout if you wish, it is not necessary
  pub fn logout(self) -> Result<()> {
    self.0.post(LOGOUT).send()?;
    Ok(())
  }

  pub fn semester_id_list(&self) -> Result<Vec<Id>> {
    let res = self.0.get(SEMESTER_LIST).send()?.json::<Vec<Option<String>>>()?; // there is `null` in response
    Ok(res.into_iter().filter_map(|x| x).collect())
  }

  pub fn course_list(&self, semester: IdRef) -> Result<Vec<Course>> {
    let mut res = self.0.get(&COURSE_LIST(semester)).send()?.json::<JsonWrapper1<Course>>()?.resultList;
    for x in &mut res {
      x.time_location = self.0.get(&COURSE_TIME_LOCATION(&x.id)).send()?.json()?;
    }
    Ok(res)
  }

  pub fn notification_list(&self, course: IdRef) -> Result<Vec<Notification>> {
    let mut res = self.0.get(&NOTIFICATION_LIST(course)).send()?.json::<JsonWrapper2<JsonWrapper20<Notification>>>()?.object.aaData;
    for x in &mut res {
      x.attachment_url = if x.attachment_name.is_some() {
        const MSG: &str = "invalid notification attachment format";
        let res = self.0.get(&NOTIFICATION_DETAIL(&x.id, course)).send()?.text()?;
        let href_end = res.find("\" class=\"ml-10\"").ok_or(MSG)?;
        let href_start = res[..href_end].rfind("a href=\"").ok_or(MSG)? + 8;
        Some(PREFIX.to_string() + &res[href_start..href_end])
      } else { None };
    }
    Ok(res)
  }

  pub fn file_list(&self, course: IdRef) -> Result<Vec<File>> {
    Ok(self.0.get(&FILE_LIST(course)).send()?.json::<JsonWrapper2<Vec<File>>>()?.object)
  }

  pub fn homework_list(&self, course: IdRef) -> Result<Vec<Homework>> {
    let mut ret = Vec::new();
    for f in &HOMEWORK_LIST_ALL {
      let mut res = self.0.get(&f(course)).send()?.json::<JsonWrapper2<JsonWrapper20<Homework>>>()?.object.aaData;
      for x in &mut res {
        let res = self.0.get(&x.detail_url()).send()?.text()?;
        x.detail = HomeworkDetail::from_html(&res).ok_or("invalid homework detail format")?;
      }
      ret.append(&mut res);
    }
    Ok(ret)
  }

  pub fn submit_homework(&self, student_homework: impl Into<Cow<'static, str>>, content: impl Into<Cow<'static, str>>,
                         file: Option<(impl Into<Cow<'static, str>>, impl Into<Cow<'static, [u8]>>)>) -> Result<()> {
    let form = Form::new().text("zynr", content).text("xszyid", student_homework).text("isDeleted", "0");
    let form = if let Some((name, data)) = file {
      form.part("fileupload", Part::bytes(data).file_name(name))
    } else { form.text("fileupload", "undefined") };
    let res = self.0.post(HOMEWORK_SUBMIT).multipart(form).send()?.text()?;
    if res.contains("success") { Ok(()) } else { Err("failed to submit homework".into()) }
  }

  pub fn discussion_list(&self, course: IdRef) -> Result<Vec<Discussion>> {
    Ok(self.0.get(&DISCUSSION_LIST(course)).send()?.json::<JsonWrapper2<JsonWrapper21<_>>>()?.object.resultsList)
  }

  pub fn question_list(&self, course: IdRef) -> Result<Vec<Question>> {
    Ok(self.0.get(&QUESTION_LIST(course)).send()?.json::<JsonWrapper2<JsonWrapper21<_>>>()?.object.resultsList)
  }
}