use anyhow::anyhow;
use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::process::exit;
use structopt::StructOpt;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, StructOpt)]
struct OptImport {
	///Pack url
	pack: String,

	///Save sticker local
	#[structopt(short, long)]
	download: bool,

	///Does not upload the sticker to Matrix
	#[structopt(short = "U", long)]
	noupload: bool,

	///Do not format the stickers;
	///The stickers can may not be shown by a matrix client
	#[structopt(short = "F", long)]
	noformat: bool,
}

#[derive(Debug, StructOpt)]
enum Opt {
	///import Stickerpack from telegram
	Import(OptImport),
}

#[derive(Deserialize)]
struct TomlFile {
	telegram_bot_key: String,
}

#[derive(Debug, Deserialize)]
struct TJsonSatet {
	ok: bool,

	error_code: Option<u32>,
	description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TJsonSticker {
	emoji: String,
	file_id: String,
}

#[derive(Debug, Deserialize)]
struct TJsonStickerPack {
	name: String,
	title: String,
	is_animated: bool,
	stickers: Vec<TJsonSticker>,
}

#[derive(Debug, Deserialize)]
struct TJsonFile {
	file_path: String,
}

fn check_telegram_resp(mut resp: serde_json::Value) -> anyhow::Result<serde_json::Value> {
	let resp_state: TJsonSatet = serde_json::from_value(resp.clone())?;
	if !resp_state.ok {
		anyhow::bail!(
			"Telegram request was not successful: {} {}",
			resp_state.error_code.unwrap(),
			resp_state.description.unwrap()
		)
	}
	let resp = resp.get_mut("result").ok_or_else(|| anyhow!("Missing 'result'"))?.take();
	Ok(serde_json::from_value(resp)?)
}

fn import(opt: OptImport) -> anyhow::Result<()> {
	let toml_file: TomlFile =
		toml::from_str(&fs::read_to_string(CONFIG_FILE).context(format!("Failed to open {}", CONFIG_FILE))?)?;
	let telegram_api_base_url: String = format!("https://api.telegram.org/bot{}", toml_file.telegram_bot_key);
	check_telegram_resp(attohttpc::get(format!("{}/getMe", telegram_api_base_url)).send()?.json()?)?;

	let stickerpack: TJsonStickerPack = serde_json::from_value(check_telegram_resp(
		attohttpc::get(format!("{}/getStickerSet", telegram_api_base_url))
			.param("name", opt.pack)
			.send()?
			.json()?,
	)?)?;
	println!("found Telegram stickerpack {}({})", stickerpack.title, stickerpack.name);
	if opt.download {
		fs::create_dir_all(format!("./stickers/{}", stickerpack.name))?;
	}
	for sticker in stickerpack.stickers {
		println!("download Sticker {} {}", sticker.emoji, sticker.file_id);
		let sticker_file: TJsonFile = serde_json::from_value(check_telegram_resp(
			attohttpc::get(format!("{}/getFile", telegram_api_base_url))
				.param("file_id", sticker.file_id)
				.send()?
				.json()?,
		)?)?;
		let sticker_image = attohttpc::get(format!(
			"https://api.telegram.org/file/bot{}/{}",
			toml_file.telegram_bot_key, sticker_file.file_path
		))
		.send()?
		.bytes()?;
		if opt.download {
			fs::write(
				std::path::Path::new(&format!("./stickers/{}", stickerpack.name))
					.join(std::path::Path::new(&sticker_file.file_path).file_name().unwrap()),
				sticker_image,
			)?;
		}
	}
	Ok(())
}

fn main() {
	let result = match Opt::from_args() {
		Opt::Import(opt) => import(opt),
	};
	match result {
		Ok(_) => (),
		Err(error) => {
			eprintln!("{:?}", error);
			exit(1)
		}
	};
}
