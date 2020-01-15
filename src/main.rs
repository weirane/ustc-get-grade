#![cfg(feature = "cli")]

use anyhow::{Context, Result};
use clap::{App, Arg};
use itertools::Itertools;
use log::{error, info};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::{thread, time::Duration};
use ustc_get_grade::blocking::get_grade;
use ustc_get_grade::Grade;

#[derive(Debug, Deserialize)]
struct Config {
    mail: Mail,
    ustc: Ustc,
}

#[derive(Debug, Deserialize)]
struct Mail {
    username: String,
    password: String,
    server: String,
    sendto: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Ustc {
    username: String,
    password: String,
    semesters: Vec<String>,
    interval: f64,
    #[serde(default)]
    send_first: bool,
}

fn get_config() -> Result<Config> {
    let options = App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .get_matches();

    let conf = options.value_of("config").unwrap_or("config.toml");
    let mut config =
        File::open(conf).with_context(|| format!("Cannot find configuration file `{}'", conf))?;
    let mut buf = String::new();
    config.read_to_string(&mut buf)?;
    let config: Config = toml::from_str(&buf)?;
    anyhow::ensure!(
        config.ustc.interval >= 10.,
        "Interval {} is too small, should >= 10.",
        config.ustc.interval
    );

    Ok(config)
}

fn run(config: &Config) -> Result<()> {
    let semesters: Vec<_> = config.ustc.semesters.iter().map(|s| s.as_str()).collect();

    info!("App started");

    let mut old_grade = get_grade(&config.ustc.username, &config.ustc.password, &semesters)?;

    if config.ustc.send_first {
        send_email(&config.mail, "Grade Report", format_grade(&old_grade))?;
    }

    loop {
        info!("Sleep for {:.1} minutes", config.ustc.interval);
        thread::sleep(Duration::from_secs_f64(60. * config.ustc.interval));

        let grade = match get_grade(&config.ustc.username, &config.ustc.password, &semesters) {
            Ok(g) => g,
            Err(e) => {
                error!("Get grade failed: {}", e);
                send_email(
                    &config.mail,
                    "Get Grade Error",
                    format!("Get grade failed: {}", e),
                )?;
                continue;
            }
        };
        if old_grade != grade {
            info!("New grade detected");
            if let Err(e) = send_email(&config.mail, "Grade Report", format_grade(&grade)) {
                error!("Send email failed: {}", e);
                send_email(
                    &config.mail,
                    "Get Grade Error",
                    format!("Send email failed: {}", e),
                )?;
                continue;
            }
            old_grade = grade;
        }
    }
}

fn format_grade(grade: &Grade) -> String {
    let preface = format!(
        "<p>Total GPA: {:.2}<br />
        Semester GPA: {:.2}<br />
        Credits earned: {:.1}<br /></p>",
        grade.gpa, grade.sem_gpa, grade.credits,
    );

    let mut grades = String::new();
    for (name, courses) in grade.scores.iter() {
        let content = courses
            .iter()
            .map(|(n, g, c)| {
                format!(
                    r#"<tr>
                    <td align="center">{}</td>
                    <td align="center">{}</td>
                    <td align="center">{}</td>
                    </tr>"#,
                    n, g, c,
                )
            })
            .join("");
        grades += &format!(
            "<h4>{}</h4>
            <table>
              <tr>
                <th>&nbsp;课程&nbsp;</th>
                <th>&nbsp;成绩&nbsp;</th>
                <th>&nbsp;学分&nbsp;</th>
              </tr>
              {}
            </table>",
            name, content
        );
    }

    preface + &grades
}

fn send_email(config: &Mail, subject: impl Into<String>, message: impl Into<String>) -> Result<()> {
    use lettre::smtp::authentication::Credentials;
    use lettre::{SmtpClient, Transport};
    use lettre_email::Email;

    info!("Sending email");

    let mut email = Email::builder()
        .from(config.username.as_str())
        .subject(subject)
        .html(message);
    for to in config.sendto.iter() {
        email = email.to(to.as_str());
    }
    let email = email.build()?;

    let cred = Credentials::new(config.username.clone(), config.password.clone());
    let mut mailer = SmtpClient::new_simple(config.server.as_str())?
        .credentials(cred)
        .transport();

    mailer.send(email.into())?;
    info!("Email sent");

    Ok(())
}

fn main() {
    env_logger::init();

    let config = get_config().unwrap_or_else(|e| {
        error!("Config error: {}", e);
        std::process::exit(1);
    });

    if let Err(e) = run(&config) {
        error!("{}", e);
        send_email(&config.mail, "Get Grade Error", format!("{}", e)).unwrap();
        std::process::exit(1);
    }
}
