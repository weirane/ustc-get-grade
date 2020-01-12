use futures::future::try_join;
use itertools::Itertools;
use log::info;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:72.0) Gecko/20100101 Firefox/72.0";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Jiaowu login failed")]
    JWLoginFailed,
    #[error("Grade is malformed")]
    GradeMalformed,
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

#[derive(Debug)]
pub struct Grade {
    /// Overall GPA
    pub gpa: f64,

    /// GPA of selected semesters
    pub sem_gpa: f64,

    /// All the credits earned
    pub credits: u64,

    /// Scores of selected semesters
    pub scores: Vec<(String, SemesterGrade)>,
}

/// Courses formated as (name, score, credit)
pub type SemesterGrade = Vec<(String, String, f64)>;

#[allow(non_snake_case)]
#[derive(serde::Deserialize, Debug)]
struct Semesters {
    id: usize,
    nameZh: String,
    nameEn: String,
    schoolYear: String,
    current: bool,
}

pub async fn get_grade(user: &str, passwd: &str, semesters: &[&str]) -> Result<Grade, Error> {
    let client = Client::builder()
        .user_agent(UA)
        .cookie_store(true)
        .build()?;

    // Login
    let data = [
        ("model", "uplogin.jsp"),
        ("service", "https://jw.ustc.edu.cn/ucas-sso/login"),
        ("warn", ""),
        ("showCode", ""),
        ("username", user),
        ("password", passwd),
        ("button", ""),
    ];

    let res = client
        .post("https://passport.ustc.edu.cn/login")
        .form(&data)
        .send()
        .await?;
    if !res.url().as_str().contains("/home") {
        return Err(Error::JWLoginFailed);
    }
    info!("Logined");

    // Get semesters
    let res = client
        .get("https://jw.ustc.edu.cn/for-std/grade/sheet/getSemesters")
        .send()
        .await?;

    let sems: Vec<Semesters> = res.json().await?;
    info!("Semesters get");

    let ids = sems
        .iter()
        .filter(|s| semesters.contains(&s.nameZh.as_str()))
        .map(|s| s.id)
        .join(",");
    let all = client
        .get("https://jw.ustc.edu.cn/for-std/grade/sheet/getGradeList")
        .query(&[("trainTypeId", "1"), ("semesterIds", "")])
        .send();
    let sem = client
        .get("https://jw.ustc.edu.cn/for-std/grade/sheet/getGradeList")
        .query(&[("trainTypeId", "1"), ("semesterIds", &ids)])
        .send();

    let (all, sem) = try_join(all, sem).await?;
    let (all, sem) = try_join(all.text(), sem.text()).await?;
    info!("Grade get");

    let sem_map = sems.iter().map(|s| (s.id, s.nameZh.clone())).collect();
    extract_grade(all, sem, sem_map).ok_or(Error::GradeMalformed)
}

fn extract_grade(all: String, sem: String, sem_map: HashMap<usize, String>) -> Option<Grade> {
    let all: Value = serde_json::from_str(&all).ok()?;
    let sem: Value = serde_json::from_str(&sem).ok()?;

    let overview = all.get("overview")?;
    let gpa = overview.get("gpa")?.as_f64()?;
    let sem_gpa = sem.get("overview")?.get("gpa")?.as_f64()?;
    let credits = overview.get("passedCredits")?.as_u64()?;

    let mut scores = Vec::new();
    for s in sem.get("semesters")?.as_array()? {
        let name = sem_map.get(&(s.get("id")?.as_u64()? as usize))?.to_owned();
        let score = s
            .get("scores")?
            .as_array()?
            .iter()
            .map(|s| {
                Some((
                    s.get("courseNameCh")?.as_str()?.to_owned(),
                    s.get("scoreCh")?.as_str()?.to_owned(),
                    s.get("credits")?.as_f64()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        scores.push((name, score));
    }

    Some(Grade {
        gpa,
        sem_gpa,
        credits,
        scores,
    })
}

#[cfg(feature = "blocking")]
pub mod blocking {
    use super::{Error, Grade};

    #[inline]
    pub fn get_grade(user: &str, passwd: &str, semesters: &[&str]) -> Result<Grade, Error> {
        tokio::runtime::Runtime::new()
            .expect("Unable to create Tokio runtime")
            .block_on(super::get_grade(user, passwd, semesters))
    }
}
