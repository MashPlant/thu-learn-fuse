use reqwest::{blocking::{Client, ClientBuilder, multipart::{Form, Part}}};
use crate::{form_file, check_success};
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
        Some(PREFIX.to_owned() + &res[href_start..href_end])
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
        x.detail = parse_homework_detail(&res).ok_or("invalid homework detail format")?;
      }
      ret.append(&mut res);
    }
    Ok(ret)
  }

  pub fn submit_homework(&self, student_homework: IdRef<'_>, content: String, file: Option<(&str, Vec<u8>)>) -> Result<()> {
    let form = Form::new().text("zynr", content).text("xszyid", student_homework.to_owned()).text("isDeleted", "0");
    let form = form_file!(form, file);
    check_success!(b,  self.0.post(HOMEWORK_SUBMIT).multipart(form), "failed to submit homework")
  }

  pub fn discussion_list(&self, course: IdRef) -> Result<Vec<Discussion>> {
    Ok(self.0.get(&DISCUSSION_LIST(course)).send()?.json::<JsonWrapper2<JsonWrapper21<_>>>()?.object.resultsList)
  }

  pub fn discussion_replies(&self, course: IdRef<'_>, discussion: IdRef<'_>, discussion_board: IdRef<'_>) -> Result<Vec<DiscussionReply>> {
    let res = self.0.get(&DISCUSSION_REPLIES(course, discussion, discussion_board)).send()?.text()?;
    parse_discussion_replies(&res).ok_or("invalid discussion replies format".into())
  }

  pub fn reply_discussion(&self, course: IdRef<'_>, discussion: IdRef<'_>, content: String, respondent_reply: Option<IdRef<'_>>, file: Option<(&str, Vec<u8>)>) -> Result<()> {
    let form = Form::new().text("wlkcid", course.to_owned()).text("tltid", discussion.to_owned()).text("nr", content.to_owned());
    let form = form_file!(form, file);
    let form = if let Some(x) = respondent_reply { form.text("fhhid", x.to_owned()).text("_fhhid", x.to_owned()) } else { form };
    check_success!(b, self.0.post(REPLY_DISCUSSION).multipart(form), "failed to reply discussion")
  }

  pub fn delete_discussion_reply(&self, course: IdRef<'_>, reply: IdRef<'_>) -> Result<()> {
    check_success!(b, self.0.post(&DELETE_DISCUSSION_REPLY(course, reply)), "failed to delete discussion reply")
  }
}