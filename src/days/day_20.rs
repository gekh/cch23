use axum::{body::Bytes, routing::post, Router};
use git2::{Repository, Tree};
use log::info;
use reqwest::StatusCode;
use tar::Archive;
use tempfile::tempdir;

pub fn get_routes() -> Router {
    Router::new()
        .route("/20/archive_files", post(archive_files))
        .route("/20/archive_files_size", post(archive_files_size))
        .route("/20/cookie", post(cookie))
}

async fn archive_files(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 archive files started");
    let mut a = Archive::new(body.as_ref());

    Ok(a.entries().unwrap().count().to_string())
}

async fn archive_files_size(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 archive files size");
    let mut a = Archive::new(body.as_ref());

    Ok(a.entries()
        .unwrap()
        .map(|file| file.unwrap().header().size().unwrap())
        .sum::<u64>()
        .to_string())
}

async fn cookie(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 cookie started");
    let mut a = Archive::new(body.as_ref());
    let temp_dir = tempdir().unwrap();
    a.unpack(temp_dir.path()).unwrap();

    let repo = match Repository::init(temp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to init Git: {}", e),
            ))
        }
    };

    let mut n = 0;

    for _ in 1..1000 {
        let rev = match n {
            0 => "christmas".to_string(),
            n => format!("christmas@{{{}}}", n),
        };
        let head = repo.revparse_ext(rev.as_str()).unwrap().0;
        let commit = head.as_commit().unwrap();
        let id = commit.id();
        let author = commit.author().name().unwrap().to_string();

        let tree = commit.tree().unwrap();
        if christmas_walk(&tree, &repo).is_some() {
            return Ok(format!("{author} {id}"));
        }
        n += 1;
    }

    Err((StatusCode::BAD_REQUEST, "no cookie".to_string()))
}

fn christmas_walk(tree: &Tree, repo: &Repository) -> Option<()> {
    for entry in tree.iter() {
        let x = entry.to_object(&repo).unwrap();
        if let Some(tt) = x.as_tree() {
            if christmas_walk(&tt, repo).is_some() {
                return Some(());
            }
            continue;
        }

        let name = entry.name().unwrap();
        if name != "santa.txt" {
            continue;
        }
        let o = entry.to_object(&repo).unwrap().peel_to_blob().unwrap();
        let content = String::from_utf8(o.content().to_vec()).unwrap();

        if content.find("COOKIE").is_some() {
            return Some(());
        }
    }

    None
}
