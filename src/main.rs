use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use icalendar::{Alarm, Calendar, Component, Event, EventLike};
use serde_json::{json, Value};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post("https://fossunited.org/api/method/fossunited.api.dashboard.get_event_from_permalink")
        .body(json!({"permalink": "indiafoss24"}).to_string())
        .header("Content-Type", "application/json")
        .send()
        .await?;
    if res.status() != 200 {
        anyhow::bail!(
            "Unexpected status code (get_event_from_permalink): {}",
            res.status()
        );
    }
    let body = res.json::<Value>().await?;
    let name = body["message"]["name"]
        .as_str()
        .with_context(|| format!("Could not get body.message.name: {body}"))?;
    let event_location = body["message"]["event_location"]
        .as_str()
        .with_context(|| format!("Could not get body.message.event_location: {body}"))?;
    let res = client
        .post("https://fossunited.org/api/method/fossunited.api.schedule.get_event_schedule")
        .body(json!({"event_id": name}).to_string())
        .header("Content-Type", "application/json")
        .send()
        .await?;
    if res.status() != 200 {
        anyhow::bail!(
            "Unexpected status code (get_event_schedule): {}",
            res.status()
        );
    }
    let body = res.json::<Value>().await?;
    let dates = body["message"]
        .as_object()
        .with_context(|| format!("Could not get body.message: {body}"))?
        .keys();

    let mut calendar = Calendar::new();

    for date in dates {
        let locations = body["message"][date]
            .as_object()
            .with_context(|| format!("Could not get body.message.{date}: {body}"))?
            .keys();

        for location in locations {
            let events = body["message"][date][location]
                .as_array()
                .with_context(|| format!("Could not get body.message.{date}.{location}: {body}"))?;

            for event in events {
                let event = event.as_object().unwrap();
                let title = event["title"].as_str().unwrap();
                let name = event["name"].as_str().unwrap();
                let parent = event["parent"].as_str().unwrap();
                let category = event["category"].as_str().unwrap();
                let date = event["scheduled_date"].as_str().unwrap();
                let start = event["start_time"].as_str().unwrap();
                let end = event["end_time"].as_str().unwrap();
                let start_date = format!("{}T{}+05:30", date, start);
                let end_date = format!("{}T{}+05:30", date, end);
                let event = Event::new()
                    .summary(&format!("{title} - IndiaFOSS 2024"))
                    .starts(start_date.parse::<DateTime<Utc>>()?)
                    .ends(end_date.parse::<DateTime<Utc>>()?)
                    .uid(&format!("{parent}.{name}"))
                    .location(&format!("{location}, {event_location}"))
                    .description(&format!("[IndiaFOSS 2024] [{category}] {title}"))
                    .class(icalendar::Class::Private)
                    .alarm(Alarm::audio(-Duration::minutes(10)))
                    .done();
                calendar.push(event);
            }
        }
    }

    create_dir_all("docs").await?;
    let mut file = File::create("docs/calendar.ics").await?;
    file.write_all(calendar.to_string().as_bytes()).await?;

    Ok(())
}
