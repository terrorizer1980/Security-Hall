use bincode;
use chrono::Utc;
use warp::{Reply, reply::Response, http::StatusCode};
use sled::Db;

use log::info;

use crate::{
    AdminKey,
    HallEntry,
    RecordSubmission,
    generate_response
};

pub fn add_record(new_record: RecordSubmission, user: &AdminKey, record_db: &Db) -> Response {
    let new_id = record_db.generate_id().unwrap();

    // Assign this with a predictable key
    let key = format!("SI-{}", new_id);
    
    // Let the user optionally define a record date, default to today
    let record_date = match new_record.date {
        Some(submitted_date) => submitted_date,
        None => Utc::today().naive_utc()
    };

    let mut formed_record = HallEntry {
        id: new_id,
        anchor_key: None,
        reference_id: new_record.reference_id,
        affected_service: new_record.affected_service,
        date: record_date,
        summary: new_record.summary,
        reporter: new_record.reporter,
        reporter_handle: new_record.reporter_handle
    };

    formed_record.generate_anchor();

    let encoded_record = bincode::serialize(&formed_record).unwrap();
    record_db.insert(key, encoded_record).unwrap();

    let msg = format!("Report created (ID: {})", new_id);

    info!("{} by {}", &msg, user.username);  
    generate_response(&msg, StatusCode::CREATED)
}

pub fn remove_record(record_id: u64, user: &AdminKey, record_db: &Db) -> Response {
    let key = format!("SI-{}", record_id);
    if record_db.remove(key).unwrap().is_some() {
        info!("Report removed (ID: {}) by {} ", record_id, user.username);  
        warp::reply::with_status("", StatusCode::NO_CONTENT).into_response()
    } else {
        let err_msg = "The requested ID doesn't exist, please try again!";
        generate_response(err_msg, StatusCode::BAD_REQUEST)
    }
}

pub fn update_record(updated_record: RecordSubmission, user: &AdminKey, record_db: &Db) -> Response {
    let (key, current_id) = match updated_record.id {
        Some(id) => (format!("SI-{}", id), id),
        None => {
            let err_msg = "No ID was provided, try again!";
            return generate_response(err_msg, StatusCode::BAD_REQUEST)
        }
    };

    match record_db.get(&key).unwrap() {
        Some(old_record) => {
            let old_record: HallEntry = bincode::deserialize(&old_record).unwrap();

            // This assures that a record's storage key remain identical to its actual ID, so it can be found again
            if old_record.id != current_id {
                let err_msg = "The provided ID and the record's current ID do not match, try again!";
                return generate_response(err_msg, StatusCode::BAD_REQUEST)
            }
            
            // We do *not* update the anchor key, so updated records can be still found using existing links.
            // As the IDs assigned are guaranteed to be unique, it is impossible for a record
            // added later to end up with the same anchor value, as the hash would differ due to the ID difference.

            // Maybe allow the user to only send what fields they want updated?
            let new_record = bincode::serialize(&HallEntry {
                reference_id: updated_record.reference_id,
                affected_service: updated_record.affected_service,
                summary: updated_record.summary,
                reporter: updated_record.reporter,
                reporter_handle: updated_record.reporter_handle,
                ..old_record
            })
            .unwrap();
            
            record_db.insert(key, new_record).unwrap();
            
            let msg = format!("Report has been updated (ID: {})", current_id);
            
            info!("{} by {}", &msg, user.username);
            generate_response(&msg, StatusCode::OK)
        }
        None => {
            let err_msg = "The requested ID doesn't exist, please try again!";
            generate_response(err_msg, StatusCode::BAD_REQUEST)
        } 
    }
}
