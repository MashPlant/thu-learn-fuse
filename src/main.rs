#![feature(async_closure)]
#[macro_use]
extern crate log;

use fuse::{Filesystem, Request, ReplyEntry, ReplyAttr, ReplyDirectory, FileType::*, FileAttr, ReplyData, ReplyWrite, ReplyOpen};
use libc::{ENOENT, EIO, EPERM};
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};
use std::sync::Arc;
use thu_learn_helper::LearnHelper;
use thu_learn_helper::types::{Course, Homework, HomeworkDetail, Notification, File, Error};
use std::borrow::{Borrow, Cow};
use tokio::runtime::Runtime;
use futures::future::{try_join_all, try_join3};
use bytes::Bytes;
use openat::Dir;

use InoInfo::*;
use std::io::Read;

// there is no need to use BTreeMap / HashMap / IndexMap for performance
// this simple solution keeps insertion order, and is quite fast when dealing with small amount of data
type Map = Vec<(String, u64)>;

// all Map's value are pointer to other ino
enum InoInfo {
  Root { users: Map },
  // the Map key in parent variant is human-readable, and children variant may store their api-used name
  User { semesters: Map },
  Semester { courses: Map },
  // add `1` to avoid name conflict with thu_learn_helper::types::Course
  Course1 {
    course: Course,
    client: Arc<LearnHelper>,
    fetched: bool,
  },
  // an `Item` can be a homework/notification/file
  ItemList(Map),
  // Map value points to `Content` or `SubmitHomework`
  Item(Vec<(Cow<'static, str>, u64)>),
  Content(Content),
  SubmitHomework {
    student_homework_id: String,
    client: Arc<LearnHelper>,
  },
}

enum Content {
  Data(Bytes),
  Url(String, Arc<LearnHelper>),
}

impl Content {
  // if my implementation is correct, user can never read the content of the file when it is a `Url` variant
  // so its data is not important, just return empty slice here
  fn bytes(&self) -> &[u8] {
    match self { Content::Data(x) => x, Content::Url(_, _) => &[] }
  }
}

// FileSystem's ino id starts from 1, fill inos[0] with Root, though it won't be accessed
struct LearnFS {
  inos: Vec<InoInfo>,
  runtime: Runtime,
}

impl LearnFS {
  fn new() -> LearnFS {
    LearnFS {
      inos: vec![Root { users: Vec::new() }, Root { users: Vec::new() }],
      runtime: Runtime::new().unwrap(),
    }
  }
}

// all information are only valid when it is returned
// in this way, when `open` modifies the size of a file, kernel will fetch the attr of this file immediately after `open`
// so that subsequent `read` can provide correct amount of data
const TTL: Duration = Duration::from_secs(0);

// all the `time` fields are `UNIX_EPOCH`, which is 1970-1-1, a useless value
// `uid` & `gid` being 1000 means the normal user in most linux systems
fn dir_attr(ino: u64) -> FileAttr {
  FileAttr { ino, size: 0, blocks: 0, atime: UNIX_EPOCH, mtime: UNIX_EPOCH, ctime: UNIX_EPOCH, crtime: UNIX_EPOCH, kind: Directory, perm: 0o777, nlink: 2, uid: 1000, gid: 1000, rdev: 0, flags: 0 }
}

fn file_attr(ino: u64, size: u64) -> FileAttr {
  FileAttr { ino, size, blocks: 1, atime: UNIX_EPOCH, mtime: UNIX_EPOCH, ctime: UNIX_EPOCH, crtime: UNIX_EPOCH, kind: RegularFile, perm: 0o666, nlink: 1, uid: 1000, gid: 1000, rdev: 0, flags: 0 }
}

// print prompt message to stdout and read password from stdin of the given process
fn get_password(pid: u32) -> std::io::Result<String> {
  use std::fs::{File, OpenOptions};
  use std::io::{Write, BufReader, BufRead};
  // magic, use procfs to get the file handle of stdin/stdout of another process
  // in `read_file` we can also get teh cwd of another process
  let mut stdout = OpenOptions::new().write(true).open(format!("/proc/{}/fd/1", pid))?;
  stdout.write_all("请输入密码：".as_bytes())?;
  stdout.flush()?;
  let mut stdin = BufReader::new(File::open(format!("/proc/{}/fd/0", pid))?);
  let mut password = String::new();
  stdin.read_line(&mut password)?;
  Ok(password.trim().to_owned())
}

// read file content of `path`, from the cwd of the given process
fn read_file(pid: u32, path: &str) -> std::io::Result<Vec<u8>> {
  let dir = Dir::open(format!("/proc/{}/cwd", pid))?;
  let mut file = dir.open_file(path)?;
  let mut buf = Vec::new();
  file.read_to_end(&mut buf)?;
  Ok(buf)
}

// inc!(x) <=> x++
macro_rules! inc { ($x: expr) => { ($x, $x += 1).0 }; }

// $map will be Item(_), each element of $contents will be Content(_)
macro_rules! push {
  ($map: expr, $content: expr, $ino: expr, $($name: expr => $val: expr),*) => {
    $($map.push(($name.into(), inc!($ino)));
    $content.push(Content::Data($val.into()));)*
  };
}

macro_rules! try_push {
  ($map: expr, $content: expr, $ino: expr, $($name: expr => $val: expr),*) => {
    $(if let Some(val) = $val {
      $map.push(($name.into(), inc!($ino)));
      $content.push(Content::Data(val.into()));
    })*
  };
}

fn bool2str(b: bool) -> &'static str { if b { "是" } else { "否" } }

fn homework_content(h: Homework, mut i: u64, client: Arc<LearnHelper>) -> (Vec<(Cow<'static, str>, u64)>, Vec<Content>) {
  let HomeworkDetail { description, attachment_name_url, submit_attachment_name_url, grade_attachment_name_url } = h.detail;
  let (mut m, mut c) = (Vec::new(), Vec::new());
  push!(m, c, i, "描述" => description, "发布时间" => h.assign_time.to_string(), "截止时间" => h.deadline.to_string());
  try_push!(m, c, i, "提交时间" => h.submit_time.map(|x| x.to_string()), "提交内容" => h.submit_content,
    "成绩" => h.grade.map(|x| x.to_string()), "批阅时间" => h.grade_time.map(|x| x.to_string()),
    "批阅老师" => h.grader_name, "评语" => h.grade_content);
  if let Some((name, url)) = attachment_name_url {
    m.push((format!("附件：{}", name).into(), inc!(i)));
    c.push(Content::Url(url, Arc::clone(&client)));
  }
  if let Some((name, url)) = submit_attachment_name_url {
    m.push((format!("提交附件：{}", name).into(), inc!(i)));
    c.push(Content::Url(url, Arc::clone(&client)));
  }
  if let Some((name, url)) = grade_attachment_name_url {
    m.push((format!("评语附件：{}", name).into(), inc!(i)));
    c.push(Content::Url(url, Arc::clone(&client)));
  }
  m.push(("提交作业".into(), i)); // it will map to SubmitHomework, added in the caller side of `homework_content`
  (m, c)
}

fn notification_content(n: Notification, mut i: u64, client: Arc<LearnHelper>) -> (Vec<(Cow<'static, str>, u64)>, Vec<Content>) {
  let (mut m, mut c) = (Vec::new(), Vec::new());
  push!(m, c, i, "内容" => n.content, "发布时间" => n.publish_time.to_string(), "发布老师" => n.publisher, "已读" => bool2str(n.read), "重要" => bool2str(n.important));
  if let (Some(name), Some(url)) = (n.attachment_name, n.attachment_url) {
    m.push((format!("通知附件：{}", name).into(), i));
    c.push(Content::Url(url, client));
  }
  (m, c)
}

fn file_content(f: File, mut i: u64, client: Arc<LearnHelper>) -> (Vec<(Cow<'static, str>, u64)>, Vec<Content>) {
  let (mut m, mut c) = (Vec::new(), Vec::new());
  let url = f.download_url();
  push!(m, c, i, "描述" => f.description, "大小" => f.size, "上传时间" => f.upload_time.to_string(), "已读" => bool2str(!f.new),
    "重要" => bool2str(f.important), "访问次数" => f.visit_count.to_string(), "下载次数" => f.download_cunt.to_string());
  m.push(((f.title + "." + &f.file_type).into(), i));
  c.push(Content::Url(url, client));
  (m, c)
}

macro_rules! unwrap {
  ($res: expr, $reply: expr, $err: expr) => {
    match $res {
      Ok(x) => x,
      Err(e) => {
        warn!("line {}: {:?}", line!(), e);
        return $reply.error($err);
      }
    }
  };
}

const COURSE_CONTENT: [&str; 3] = ["作业", "通知", "文件"];
// a course takes `COURSE_INO` inos, 1 for itself, remaining for its contents
const COURSE_INO: u64 = COURSE_CONTENT.len() as u64 + 1;

impl Filesystem for LearnFS {
  fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    info!("lookup parent={} name={:?}", parent, name);
    fn do_lookup<S: Borrow<str>>(m: impl IntoIterator<Item=impl Borrow<(S, u64)>>, k: &str) -> Option<u64> {
      m.into_iter().find(|x| x.borrow().0.borrow() == k).map(|x| x.borrow().1)
    }
    let ref name = name.to_string_lossy();
    fn reply_map<S: Borrow<str>>(m: impl IntoIterator<Item=impl Borrow<(S, u64)>>, name: &str, reply: ReplyEntry) {
      if let Some(ino) = do_lookup(m, &name) {
        reply.entry(&TTL, &dir_attr(ino), 0);
      } else { reply.error(ENOENT); }
    }
    match &mut self.inos[parent as usize] {
      Root { users: m } | User { semesters: m, .. } | Semester { courses: m } | ItemList(m) =>
        reply_map(m, name, reply),
      Item(m) =>
        if let Some(ino) = do_lookup(m, &name) {
          match &self.inos[ino as usize] {
            Content(c) => reply.entry(&TTL, &file_attr(ino, c.bytes().len() as u64), 0),
            SubmitHomework { .. } => reply.entry(&TTL, &file_attr(ino, 0), 0),
            _ => unreachable!(),
          }
        } else { reply.error(ENOENT); }
      // going into any child dir representing course content must first call `lookup`, so fill the content of them here
      Course1 { course, client, fetched } => {
        if !*fetched {
          let (hs, ns, fs) = unwrap!(self.runtime.block_on(try_join3(
            client.homework_list(&course.id),
            client.notification_list(&course.id),
            client.file_list(&course.id))), reply, EIO);
          *fetched = true;
          let client = Arc::clone(client);
          macro_rules! handle_items {
            ($name: ident, $items: expr, $content_fn: expr, $offset: expr, $op1: stmt, $op2: stmt) => {
              #[allow(redundant_semicolons)]
              #[allow(unused_mut)]
              for mut $name in $items {
                let new_ino = self.inos.len() as u64;
                let name = $name.title.clone();
                $op1;
                let (m, cs) = $content_fn($name, new_ino + 1, Arc::clone(&client));
                self.inos.push(Item(m));
                for c in cs { self.inos.push(Content(c)); }
                $op2;
                match &mut self.inos[parent as usize + $offset] {
                  ItemList(m) => m.push((name, new_ino)), _ => unreachable!()
                }
              }
            };
          }
          handle_items!(x, hs, homework_content, 1,
            let student_homework_id = std::mem::replace(&mut x.student_homework_id, String::new()),
            self.inos.push(SubmitHomework { student_homework_id, client: Arc::clone(&client) }));
          handle_items!(x, ns, notification_content, 2, {}, {});
          handle_items!(x, fs, file_content, 3, {}, {});
        }
        reply_map(COURSE_CONTENT.iter().copied().zip(parent + 1..), name, reply);
      }
      Content(_) | SubmitHomework { .. } => reply.error(EPERM),
    }
  }

  fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
    info!("getattr ino={}", ino);
    match &self.inos[ino as usize] {
      Root { .. } | User { .. } | Semester { .. } | Course1 { .. } | ItemList(_) | Item(_) => reply.attr(&TTL, &dir_attr(ino)),
      Content(c) => reply.attr(&TTL, &file_attr(ino, c.bytes().len() as u64)),
      SubmitHomework { .. } => reply.attr(&TTL, &file_attr(ino, 0)),
    }
  }

  fn mkdir(&mut self, req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
    info!("mkdir parent={} name={:?}", parent, name);
    let name = name.to_string_lossy();
    let new_ino = self.inos.len() as u64;
    match &mut self.inos[parent as usize] {
      Root { users } => {
        // we don't need to worry about duplication here, because a prior `lookup` call will prevent it
        let password = unwrap!(get_password(req.pid()), reply, EIO);
        let tmp = self.runtime.block_on(async {
          let cl = LearnHelper::login(&name, &password).await?;
          let ss = cl.semester_id_list().await?;
          let cl1 = &cl;
          let css = try_join_all(ss.iter().map(async move |s| cl1.course_list(s).await)).await?;
          Ok::<_, Error>((Arc::new(cl), ss, css))
        });
        let (cl, ss, css) = unwrap!(tmp, reply, EIO);
        let ss1 = ss.iter().map(|s| {
          let (l, r) = s.split_at(s.len() - 1);
          l.to_owned() + match r { "1" => "秋", "2" => "春", "3" => "夏", _ => panic!("invalid semester type"), }
        }).zip(css.iter().scan(new_ino + 1, |sum, cs| (Some(*sum), *sum += cs.len() as u64 * COURSE_INO + 1).0))
          .collect();
        users.push((name.into_owned(), new_ino));
        self.inos.push(User { semesters: ss1 });
        for cs in css {
          let new_ino = self.inos.len() as u64;
          let cs1 = cs.iter().map(|c| c.name.clone()).zip((new_ino + 1..).step_by(4)).collect();
          self.inos.push(Semester { courses: cs1 });
          for course in cs {
            self.inos.push(Course1 { course, client: Arc::clone(&cl), fetched: false });
            self.inos.push(ItemList(Vec::new()));
            self.inos.push(ItemList(Vec::new()));
            self.inos.push(ItemList(Vec::new()));
          }
        }
        reply.entry(&TTL, &dir_attr(new_ino), 0);
      }
      _ => reply.error(EPERM),
    }
  }

  fn open(&mut self, _req: &Request, ino: u64, _flags: u32, reply: ReplyOpen) {
    info!("open ino={}", ino);
    if let Content(Content::Url(url, client)) = &mut self.inos[ino as usize] {
      self.inos[ino as usize] = Content(Content::Data(unwrap!(self.runtime.block_on(async {
        client.0.get(url.as_str()).send().await?.bytes().await
      }), reply, EIO)));
    }
    // this is the default implementation of `FileSystem`, returning such data makes the `open` operation useless
    reply.opened(0, 0);
  }

  fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
    info!("read ino={} offset={} size={}", ino, offset, size);
    let offset = offset as usize;
    match &self.inos[ino as usize] {
      Content(c) => {
        let data = c.bytes();
        reply.data(&data[offset..(offset + size as usize).min(data.len())])
      }
      SubmitHomework { .. } => reply.data(&[]),
      _ => reply.error(EPERM),
    }
  }

  fn write(&mut self, req: &Request, ino: u64, _fh: u64, _offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
    info!("write ino={} data={:?}", ino, data);
    match &mut self.inos[ino as usize] {
      SubmitHomework { student_homework_id, client } => {
        reply.written(data.len() as u32);
        // the operation of fuse is not re-entrant, so we must finish `write` before we can start another operation
        // I choose to spawn the handle finish this request, so that error handling must be ignored, because their is no way to fetch the result
        let (pid, data) = (req.pid(), data.to_vec());
        let (student_homework_id, client) = (student_homework_id.clone(), Arc::clone(client));
        self.runtime.spawn(async move {
          let data = if let Ok(x) = std::str::from_utf8(&data) { x } else { return; };
          let (file, content) = if data.starts_with("FILE=") {
            let data = &data[5..];
            let file_end = data.find(|x: char| x.is_whitespace()).unwrap_or(data.len());
            (Some(&data[..file_end]), &data[file_end..])
          } else { (None, data) };
          let file = if let Some(f) = file {
            Some((f, if let Ok(x) = read_file(pid, f) { x } else { return; }))
          } else { None };
          let _ = client.submit_homework(&student_homework_id, &content, file).await;
        });
      }
      _ => reply.error(EPERM),
    }
  }

  fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
    info!("readdir ino={} offset={}", ino, offset);
    if offset < 1 { reply.add(ino, 1, Directory, "."); }
    if offset < 2 { reply.add(ino, 2, Directory, ".."); }
    fn reply_map<S: Borrow<str>>(m: impl IntoIterator<Item=impl Borrow<(S, u64)>>, offset: i64, mut reply: ReplyDirectory) {
      for (idx, x) in m.into_iter().enumerate().skip((offset - 2).max(0) as usize) {
        let (id, ino) = x.borrow();
        reply.add(*ino, (idx + 3) as i64, Directory, id.borrow());
      }
      reply.ok();
    }
    match &mut self.inos[ino as usize] {
      Root { users: m } | User { semesters: m, .. } | Semester { courses: m } | ItemList(m) => reply_map(m, offset, reply),
      Item(m) => reply_map(m, offset, reply),
      Course1 { .. } => reply_map(COURSE_CONTENT.iter().copied().zip(ino + 1..), offset, reply),
      _ => reply.error(EPERM),
    }
  }
}

fn main() {
  env_logger::init();
  let mountpoint = if let Some(x) = std::env::args_os().nth(1) { x } else {
    eprintln!("使用方法： <程序> <挂载点>");
    std::process::exit(1);
  };
  fuse::mount(LearnFS::new(), mountpoint, &[]).unwrap();
}