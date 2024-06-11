use std::{net::SocketAddr, str::FromStr};

use axum::{response::IntoResponse, Router};
const TEST_HTML:&'static str=r#"""
<html><head>
<meta property="og:description" content="description text">
<title>TEST_HTML_FILE</title>
</head></html>
"""#;

#[derive(Debug,serde::Serialize,serde::Deserialize)]
pub struct SummalyResult{
	title:String,
	description:String,
}
fn main() {
	let args:Vec<String>=std::env::args().collect();
	let bind_port=args.get(1).expect("args[1]=bind_port");
	let target_url=args.get(2).expect("args[2]=target_url");
	let http_addr:SocketAddr = SocketAddr::new("127.0.0.1".parse().unwrap(),bind_port.parse().expect("bind_port parse"));
	let self_url=reqwest::Url::from_str(&format!("http://{}:{}/",http_addr.ip().to_string(),http_addr.port())).unwrap();
	let rt=tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
	rt.spawn(async move{
		let app = Router::new();
		let app=app.route("/",axum::routing::get(||async{
			(axum::http::StatusCode::OK,TEST_HTML).into_response()
		}));
		let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
		axum::serve(listener,app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
	});
	let client=reqwest::Client::builder();
	let client=client.timeout(std::time::Duration::from_millis(500));
	let client=client.build().unwrap();
	let mut local_ok=false;
	for _ in 0..20{
		std::thread::sleep(std::time::Duration::from_millis(50));
		let self_url=self_url.clone();
		let client=client.clone();
		let status=rt.block_on(async move{
			if let Ok(s)=client.get(self_url).send().await{
				s.status().as_u16()
			}else{
				504
			}
		});
		if status==200{
			local_ok=true;
			break;
		}
		std::thread::sleep(std::time::Duration::from_millis(50));
	}
	if !local_ok{
		println!("test server bind error");
		std::process::exit(1);
	}
	for _ in 0..5{
		let self_url=self_url.to_string();
		let client=client.clone();
		let status=rt.block_on(async move{
			if let Ok(s)=client.get(format!("{}?url={}",target_url,self_url)).send().await{
				let status_code=s.status().as_u16();
				if status_code==200{
					let s=s.bytes().await.expect("body load failed");
					let sr=serde_json::from_slice::<SummalyResult>(&s).unwrap();
					assert_eq!(sr.description,"description text");
					assert_eq!(sr.title,"TEST_HTML_FILE");
				}
				status_code
			}else{
				504
			}
		});
		if status==200{
			println!("ok");
			std::process::exit(0);
		}
		std::thread::sleep(std::time::Duration::from_millis(500));
	}
	std::process::exit(2);
}
