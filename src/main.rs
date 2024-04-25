use std::{io::Write, net::SocketAddr, sync::Arc};

use axum::{response::IntoResponse, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ConfigFile{
	bind_addr: String,
	timeout:u64,
	user_agent:String,
	max_size:u32,
	proxy:Option<String>,
	media_proxy:Option<String>,
	append_headers:Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct RequestParams{
	url: String,
	lang:Option<String>,
	#[serde(rename = "userAgent")]
	user_agent:Option<String>,
	#[serde(rename = "responseTimeout")]
	response_timeout:Option<u32>,
	#[serde(rename = "contentLengthLimit")]
	content_length_limit:Option<u32>,
}
#[derive(Debug,Serialize,Deserialize)]
pub struct SummalyPlayer{
	url:Option<String>,
	width:Option<f64>,
	height:Option<f64>,
	allow:Vec<String>,
}
#[derive(Debug,Serialize,Deserialize)]
pub struct SummalyResult{
	url:String,
	title:Option<String>,
	icon:Option<String>,
	description:Option<String>,
	thumbnail:Option<String>,
	sitename:Option<String>,
	player:Option<SummalyPlayer>,
	sensitive:bool,
	#[serde(rename = "activityPub")]
	activity_pub:Option<String>,
	oembed:Option<OEmbed>,
}
#[derive(Debug,Serialize,Deserialize)]
pub struct OEmbed{
	r#type:String,
	version:String,
	title:Option<String>,
	author_name:Option<String>,
	author_url:Option<String>,
	provider_name:Option<String>,
	provider_url:Option<String>,
	cache_age:Option<f64>,
	thumbnail_url:Option<String>,
	thumbnail_width:Option<f64>,
	thumbnail_height:Option<f64>,
	url:Option<String>,//type=photo
	html:Option<String>,//type=video/rich
	width:Option<f64>,
	height:Option<f64>,
}
fn main() {
	let config_path=match std::env::var("SUMMALY_CONFIG_PATH"){
		Ok(path)=>{
			if path.is_empty(){
				"config.json".to_owned()
			}else{
				path
			}
		},
		Err(_)=>"config.json".to_owned()
	};
	if !std::path::Path::new(&config_path).exists(){
		let default_config=ConfigFile{
			bind_addr: "0.0.0.0:12267".to_owned(),
			timeout:5000,
			user_agent: "https://github.com/yojo-art/summaly-rs".to_owned(),
			max_size:2*1024*1024,
			proxy:None,
			media_proxy:None,//e.g. https://misskey.example.com/proxy/
			append_headers:[
				"Content-Security-Policy:default-src 'none'; img-src 'self'; media-src 'self'; style-src 'unsafe-inline'".to_owned(),
				"Access-Control-Allow-Origin:*".to_owned(),
			].to_vec(),
		};
		let default_config=serde_json::to_string_pretty(&default_config).unwrap();
		std::fs::File::create(&config_path).expect("create default config.json").write_all(default_config.as_bytes()).unwrap();
	}
	let config:ConfigFile=serde_json::from_reader(std::fs::File::open(&config_path).unwrap()).unwrap();
	let config=Arc::new(config);
	let client=reqwest::ClientBuilder::new();
	let client=client.build().unwrap();
	let rt=tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
	let arg_tup=(client,config);
	rt.block_on(async{
		let http_addr:SocketAddr = arg_tup.1.bind_addr.parse().unwrap();
		let app = Router::new();
		let arg_tup0=arg_tup.clone();
		let app=app.route("/",axum::routing::get(move|headers,parms|get_file(None,headers,arg_tup0.clone(),parms)));
		let app=app.route("/*path",axum::routing::get(move|path,headers,parms|get_file(Some(path),headers,arg_tup.clone(),parms)));
		let comression_layer= tower_http::compression::CompressionLayer::new()
		.gzip(true);
		let app=app.layer(comression_layer);
		let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
		axum::serve(listener,app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
	});
}
async fn get_file(
	_path:Option<axum::extract::Path<String>>,
	request_headers:axum::http::HeaderMap,
	(client,config):(reqwest::Client,Arc<ConfigFile>),
	axum::extract::Query(q):axum::extract::Query<RequestParams>,
)->axum::response::Response{
	println!("{:?}",q);
	let builder=client.get(&q.url);
	let user_agent=q.user_agent.as_ref().unwrap_or_else(||&config.user_agent);
	let builder=builder.header(reqwest::header::USER_AGENT,user_agent);
	let builder=if let Some(lang)=q.lang{
		builder.header(reqwest::header::ACCEPT_LANGUAGE,lang)
	}else{
		builder
	};
	let timeout_ms=config.timeout.min(q.response_timeout.unwrap_or(u32::MAX) as u64);
	let builder=builder.timeout(std::time::Duration::from_millis(timeout_ms));
	let content_length_limit=q.content_length_limit.unwrap_or(config.max_size);
	let resp=builder.send().await;
	let resp=match resp{
		Ok(resp)=>resp,
		Err(e)=>return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,e.to_string()).into_response(),
	};
	let v=match load_all(resp,content_length_limit.into()).await{
		Ok(v)=>v,
		Err(e)=>return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,e).into_response(),
	};
	//strはutf8表現なのでゼロコピー操作
	let s=match std::str::from_utf8(&v){
		Ok(s)=>s,
		Err(e)=>return (axum::http::StatusCode::BAD_GATEWAY,e.to_string()).into_response(),
	};
	std::fs::write("wip2.html",s).unwrap();
	let start=match s.find("<head"){
		Some(idx)=>idx,
		None=>return (axum::http::StatusCode::BAD_GATEWAY,"no head").into_response(),
	};
	let end=match s.find("</head>"){
		Some(idx)=>idx,
		None=>return (axum::http::StatusCode::BAD_GATEWAY,"no /head").into_response(),
	};
	let s=&s[start+6..end];
	std::fs::write("wip",s.as_bytes()).unwrap();
	let dom=match html_parser::Dom::parse(s){
		Ok(idx)=>idx,
		Err(e)=>return (axum::http::StatusCode::BAD_GATEWAY,e.to_string()).into_response(),
	};
	std::fs::write("wip.json",dom.to_json_pretty().unwrap()).unwrap();
	let base_url=if let Ok(url)=reqwest::Url::parse(&q.url){
		format!("{}://{}{}",url.scheme(),url.host_str().unwrap(),url.port().map(|n|format!(":{n}")).unwrap_or_default())
	}else{
		"https://localhost".to_owned()
	};
	let mut player=SummalyPlayer{
		url: None,
		width: None,
		height: None,
		allow: vec![],
	};
	let mut resp=SummalyResult{
		title: None,
		icon: Some(format!("{}/favicon.ico",base_url)),
		description: None,
		thumbnail: None,
		sitename: None,
		player: None,
		sensitive: false,
		activity_pub: None,
		url: q.url.clone(),
		oembed:None,
	};
	for node in dom.children.iter(){
		if let html_parser::Node::Element(element)=node{
			match (element.name.as_str(),&element.attributes){
				("meta",att)=>{
					match att.get("property").unwrap_or(&None).as_ref().map(|s|(
						s.as_str(),
						att.get("content").unwrap_or(&None).as_ref(),
					)){
							Some(("og:image",Some(content))) => {
								resp.thumbnail=Some(content.clone());
							},
							Some(("og:url",Some(content))) => {
								resp.url=content.clone();
							},
							Some(("og:title",Some(content))) => {
								resp.title=Some(content.clone());
							},
							Some(("og:description",Some(content))) => {
								resp.description=Some(content.clone());
							},
							Some(("description",Some(content))) => {
								resp.description=Some(content.clone());
							},
							Some(("og:site_name",Some(content))) => {
								resp.sitename=Some(content.clone());
							},
							Some(("og:video:url",Some(content))) => {
								if player.url.is_none(){//og:video:secure_url優先
									player.url=Some(content.clone());
								}
							},
							Some(("og:video:secure_url",Some(content))) => {
								player.url=Some(content.clone());
							},
							Some(("og:video:width",Some(content))) => {
								if let Ok(content)=content.parse::<f64>(){
									player.width=Some(content);
								}
							},
							Some(("og:video:height",Some(content))) => {
								if let Ok(content)=content.parse::<f64>(){
									player.height=Some(content);
								}
							},
							_ => {},
						}
				},
				("link",att)=>{
					match att.get("rel").unwrap_or(&None).as_ref().map(|s|(
						s.as_str(),
						att.get("href").unwrap_or(&None).as_ref(),
						att.get("type").unwrap_or(&None).as_ref().map(|t|t.as_str()),
					)){
							Some(("shortcut icon",Some(href),_)) => {
								if resp.icon.is_none(){//icon優先
									resp.icon=Some(href.clone());
								}
							},
							Some(("icon",Some(href),_)) => {
								resp.icon=Some(href.clone());
							},
							Some(("apple-touch-icon",Some(href),_)) => {
								if resp.thumbnail.is_none(){//og:image優先
									resp.thumbnail=Some(href.clone());
								}
							},
							Some(("alternate",Some(href),Some("application/json+oembed"))) => {
								let href=html_escape::decode_html_entities(&href);
								let embed_res=if let Ok(href)=urlencoding::decode(&href){
									let builder=client.get(href.as_ref());
									let user_agent=q.user_agent.as_ref().unwrap_or_else(||&config.user_agent);
									let builder=builder.header(reqwest::header::USER_AGENT,user_agent);
									let timeout_ms=config.timeout.min(q.response_timeout.unwrap_or(u32::MAX) as u64);
									let builder=builder.timeout(std::time::Duration::from_millis(timeout_ms));
									builder.send().await.ok()
								}else{
									None
								};
								let embed_json=if let Some(embed_res)=embed_res{
									if let Ok(d)=load_all(embed_res,content_length_limit.into()).await{
										serde_json::from_slice(&d).ok()
									}else{
										None
									}
								}else{
									None
								};
								if let Some(v)=embed_json{
									resp.oembed=Some(v);
								}
							},
							_ => {},
						}
				},
				_=>{}
			}
		}
	}
	if let Some(v)=&resp.oembed{
		if let Some(width)=v.width{
			player.width=Some(width);
		}
		if let Some(height)=v.height{
			player.height=Some(height);
		}
		const SAFE_LIST:[&'static str;6] = [
			"autoplay",
			"clipboard-write",
			"fullscreen",
			"encrypted-media",
			"picture-in-picture",
			"web-share",
		];
		if let Some(html)=v.html.as_ref().map(|v|v.as_str()){
			if let Ok(html)=html_parser::Dom::parse(html){
				for node in html.children.iter(){
					if let html_parser::Node::Element(node)=node{
						if let Some(Some(allow))=node.attributes.get("allow"){
							for allow in allow.split(";"){
								let allow=allow.trim();
								if SAFE_LIST.contains(&allow){
									player.allow.push(allow.to_owned());
								}
							}
						}
					}
				}
			}
		}
	}
	//すべての有効なプレイヤーにはurlが存在する
	if player.url.is_some(){
		resp.player=Some(player);
	}
	if let Some(icon)=&resp.icon{
		if icon.starts_with("/"){
			resp.icon=Some(format!("{}{}",base_url,icon));
		}
		if let Some(media_proxy)=&config.media_proxy{
			resp.icon=Some(format!("{}icon.webp?url={}",media_proxy,urlencoding::encode(resp.icon.as_ref().unwrap())));
		}
	}
	if let Some(thumbnail)=&resp.thumbnail{
		if thumbnail.starts_with("/"){
			resp.thumbnail=Some(format!("{}{}",base_url,thumbnail));
		}
		if let Some(media_proxy)=&config.media_proxy{
			resp.thumbnail=Some(format!("{}thumbnail.webp?url={}",media_proxy,urlencoding::encode(resp.thumbnail.as_ref().unwrap())));
		}
	}
	if let Ok(json)=serde_json::to_string(&resp){
		let mut headers=axum::http::HeaderMap::new();
		headers.append("Content-Type","application/json".parse().unwrap());
		(axum::http::StatusCode::OK,headers,json).into_response()
	}else{
		axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
	}
}
async fn load_all(resp: reqwest::Response,content_length_limit:u64)->Result<Vec<u8>,String>{
	let len_hint=resp.content_length().unwrap_or(content_length_limit);
	if len_hint>content_length_limit{
		return Err(format!("lengthHint:{}>{}",len_hint,content_length_limit));
	}
	let mut response_bytes=Vec::with_capacity(len_hint as usize);
	let mut stream=resp.bytes_stream();
	while let Some(x) = stream.next().await{
		match x{
			Ok(b)=>{
				if response_bytes.len()+b.len()>content_length_limit as usize{
					return Err(format!("length:{}>{}",response_bytes.len()+b.len(),content_length_limit))
				}
				response_bytes.extend_from_slice(&b);
			},
			Err(e)=>{
				return Err(format!("LoadAll:{:?}",e))
			}
		}
	}
	Ok(response_bytes)
}
