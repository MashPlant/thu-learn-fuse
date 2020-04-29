pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.149 Safari/537.36";

pub const PREFIX: &str = "https://learn.tsinghua.edu.cn";
pub const LOGIN: &str = "https://id.tsinghua.edu.cn/do/off/ui/auth/login/post/bb5df85216504820be7bba2b0ae1535b/0?/login.do";

pub fn AUTH_ROAM(ticket: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/j_spring_security_thauth_roaming_entry?ticket={}", ticket)
}

pub const LOGOUT: &str = "https://learn.tsinghua.edu.cn/f/j_spring_security_logout";
pub const SEMESTER_LIST: &str = "https://learn.tsinghua.edu.cn/b/wlxt/kc/v_wlkc_xs_xktjb_coassb/queryxnxq";

pub fn COURSE_LIST(semester: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kc/v_wlkc_xs_xkb_kcb_extend/student/loadCourseBySemesterId/{}", semester)
}

pub fn COURSE_URL(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/f/wlxt/index/course/student/course?wlkcid={}", course)
}

pub fn COURSE_TIME_LOCATION(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/kc/v_wlkc_xk_sjddb/detail?id={}", course)
}

pub fn FILE_LIST(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kj/wlkc_kjxxb/student/kjxxbByWlkcidAndSizeForStudent?wlkcid={}&size=200", course)
}

pub fn FILE_DOWNLOAD(file: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kj/wlkc_kjxxb/student/downloadFile?sfgk=0&wjid={}", file)
}

pub fn NOTIFICATION_LIST(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kcgg/wlkc_ggb/student/kcggListXs?wlkcid={}&size=200", course)
}

pub fn NOTIFICATION_DETAIL(notification: &str, course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/f/wlxt/kcgg/wlkc_ggb/student/beforeViewXs?wlkcid={}&id={}", course, notification)
}

pub fn HOMEWORK_LIST_NEW(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kczy/zy/student/index/zyListWj?wlkcid={}&size=200", course)
}

pub fn HOMEWORK_LIST_SUBMITTED(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kczy/zy/student/index/zyListYjwg?wlkcid={}&size=200", course)
}

pub fn HOMEWORK_LIST_GRADED(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/kczy/zy/student/index/zyListYpg?wlkcid={}&size=200", course)
}

pub const HOMEWORK_LIST_ALL: [fn(&str) -> String; 3] = [HOMEWORK_LIST_NEW, HOMEWORK_LIST_SUBMITTED, HOMEWORK_LIST_GRADED];

pub fn HOMEWORK_DETAIL(course: &str, homework: &str, student_homework: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/f/wlxt/kczy/zy/student/viewCj?wlkcid={}&zyid={}&xszyid={}", course, homework, student_homework)
}

// the page that you click "submit homework" in browser, not really used in submitting homework
pub fn HOMEWORK_SUBMIT_PAGE(course: &str, student_homework: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/f/wlxt/kczy/zy/student/tijiao?wlkcid={}&xszyid={}", course, student_homework)
}

// the url that really performs submitting
pub const HOMEWORK_SUBMIT: &str = "https://learn.tsinghua.edu.cn/b/wlxt/kczy/zy/student/tjzy";

pub fn DISCUSSION_LIST(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/bbs/bbs_tltb/student/kctlList?wlkcid={}&size=200", course)
}

pub fn QUESTION_LIST(course: &str) -> String {
  format!("https://learn.tsinghua.edu.cn/b/wlxt/bbs/bbs_tltb/student/kcdyList?wlkcid={}&size=200", course)
}