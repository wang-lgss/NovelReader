//! 番茄小说接口：Web 免签（搜索/详情/目录）+ App 签名（正文章节）。
//! 签名算法移植自 TomatoNovel 的纯 Python 实现（SM3 + Simon + AES-CBC / Speck）。
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use base64::Engine as _;
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};

const APP_BASE: &str = "https://api5-normal-sinfonlineb.fqnovel.com";
const WEB_BASE: &str = "https://fanqienovel.com";
const REG_KEY: &str = "ac25c67ddd8f38c1b37a2348828e222e";
const UA_WEB: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const UA_APP: &str = "com.dragon.read.oversea.gp/68132 (Linux; U; Android 10; zh_CN; OnePlus11; Build/V291IR;tt-ok/3.12.13.4-tiktok)";

const SIGN_KEY: [u8; 32] = [
    0xac, 0x1a, 0xda, 0xae, 0x95, 0xa7, 0xaf, 0x94, 0xa5, 0x11, 0x4a, 0xb3, 0xb3, 0xa9, 0x7d, 0xd8,
    0x00, 0x50, 0xaa, 0x0a, 0x39, 0x31, 0x4c, 0x40, 0x52, 0x8c, 0xae, 0xc9, 0x52, 0x56, 0xc2, 0x8c,
];
const LOW_RAND: [u8; 2] = [0xF2, 0x81];
const HIGH_RAND: [u8; 2] = [0x61, 0x6F];
const XOR_PREFIX: [u8; 8] = [0xf2, 0xf7, 0xfc, 0xff, 0xf2, 0xf7, 0xfc, 0xff];
const HEADER: [u8; 9] = [0xa6, 0x6e, 0xad, 0x9f, 0x77, 0x01, 0xd0, 0x0c, 0x18];

const DEVICE_STATIC: &[(&str, &str)] = &[
    ("ac", "wifi"),
    ("channel", "43536163a"),
    ("aid", "1967"),
    ("app_name", "novelapp"),
    ("version_code", "68132"),
    ("version_name", "6.8.1.32"),
    ("device_platform", "android"),
    ("os", "android"),
    ("ssmix", "a"),
    ("language", "zh"),
    ("manifest_version_code", "68132"),
    ("update_version_code", "68132"),
    ("host_abi", "arm64-v8a"),
    ("dragon_device_type", "0"),
    ("pv_player", "1"),
    ("compliance_status", "0"),
    ("need_personal_recommend", "0"),
    ("player_so_load", "1"),
];

#[derive(Clone, Serialize, Deserialize)]
struct DeviceModel {
    device_model: String,
    device_brand: String,
    device_manufacturer: String,
    os_version: String,
    os_api: u32,
    resolution: String,
    dpi: u32,
    rom_version: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct DeviceIdentity {
    device_id: String,
    install_id: String,
    device_token: String,
    cdid: String,
    openudid: String,
    region: String,
    model: DeviceModel,
    created_at: i64,
}

struct DeviceProfile {
    device_model: &'static str,
    device_brand: &'static str,
    device_manufacturer: &'static str,
    os_version: &'static str,
    os_api: u32,
    resolution: &'static str,
    dpi: u32,
    rom_version: &'static str,
}

const DEVICE_PROFILES: &[DeviceProfile] = &[
    DeviceProfile {
        device_model: "OnePlus11",
        device_brand: "OnePlus",
        device_manufacturer: "OnePlus",
        os_version: "10",
        os_api: 29,
        resolution: "3200*1440",
        dpi: 640,
        rom_version: "V291IR+release-keys",
    },
    DeviceProfile {
        device_model: "P30",
        device_brand: "HUAWEI",
        device_manufacturer: "HUAWEI",
        os_version: "10",
        os_api: 29,
        resolution: "1080*2340",
        dpi: 480,
        rom_version: "ELS-AN00",
    },
    DeviceProfile {
        device_model: "MI 9",
        device_brand: "Xiaomi",
        device_manufacturer: "Xiaomi",
        os_version: "10",
        os_api: 29,
        resolution: "1080*2340",
        dpi: 440,
        rom_version: "MIUI11",
    },
    DeviceProfile {
        device_model: "Pixel 4",
        device_brand: "google",
        device_manufacturer: "Google",
        os_version: "11",
        os_api: 30,
        resolution: "1080*2280",
        dpi: 440,
        rom_version: "RQ3A.210805.001",
    },
    DeviceProfile {
        device_model: "SM-G9860",
        device_brand: "samsung",
        device_manufacturer: "samsung",
        os_version: "11",
        os_api: 30,
        resolution: "1440*3200",
        dpi: 560,
        rom_version: "RP1A.200720.012",
    },
];

static DEVICE_IDENTITY: OnceLock<Result<DeviceIdentity, String>> = OnceLock::new();

fn device_identity() -> Result<&'static DeviceIdentity, String> {
    DEVICE_IDENTITY
        .get_or_init(load_or_create_device_identity)
        .as_ref()
        .map_err(|e| e.clone())
}

fn device_identity_file() -> PathBuf {
    crate::shared::data_root().join("fanqie-device.json")
}

fn load_or_create_device_identity() -> Result<DeviceIdentity, String> {
    let file = device_identity_file();
    if let Ok(text) = fs::read_to_string(&file) {
        if let Ok(identity) = serde_json::from_str::<DeviceIdentity>(&text) {
            if valid_device_identity(&identity) {
                return Ok(identity);
            }
        }
    }

    let identity = register_device_identity()?;
    if let Ok(json) = serde_json::to_string_pretty(&identity) {
        if let Some(parent) = file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&file, json);
    }
    Ok(identity)
}

fn valid_device_identity(identity: &DeviceIdentity) -> bool {
    !identity.device_id.is_empty()
        && identity.device_id.parse::<u64>().is_ok()
        && !identity.install_id.is_empty()
        && identity.install_id.parse::<u64>().is_ok()
        && identity.cdid.len() == 36
        && identity.openudid.len() == 16
}

fn new_cdid() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

fn new_openudid() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn register_device_identity() -> Result<DeviceIdentity, String> {
    const REGISTER_URLS: &[&str] = &[
        "https://log.snssdk.com/service/2/device_register/",
        "https://log.isnssdk.com/service/2/device_register/",
    ];

    let profile = &DEVICE_PROFILES[rand::thread_rng().gen_range(0..DEVICE_PROFILES.len())];
    let cdid = new_cdid();
    let openudid = new_openudid();
    let body = serde_json::json!({
        "magic_tag": "ss_app_log",
        "header": {
            "display_name": "novelapp",
            "update_version_code": 68132,
            "manifest_version_code": 68132,
            "aid": 1967,
            "channel": "43536163a",
            "package": "com.dragon.read",
            "app_name": "novelapp",
            "version_code": 68132,
            "version_name": "6.8.1.32",
            "device_platform": "android",
            "language": "zh",
            "region": "CN",
            "cdid": cdid,
            "openudid": openudid,
            "device_model": profile.device_model,
            "device_brand": profile.device_brand,
            "device_manufacturer": profile.device_manufacturer,
            "os_version": profile.os_version,
            "os_api": profile.os_api,
            "resolution": profile.resolution,
            "dpi": profile.dpi,
            "rom_version": profile.rom_version,
        },
        "_gen_time": now_secs(),
    });
    let body = serde_json::to_vec(&body).map_err(|e| format!("构建设备注册请求失败: {}", e))?;
    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers.insert("User-Agent".into(), "okhttp/3.12.13".into());

    let mut last_err = String::from("未知错误");
    for url in REGISTER_URLS {
        match http("POST", url, &headers, Some(&body)) {
            Ok(raw) => {
                let v = match json_value(&raw) {
                    Ok(v) => v,
                    Err(e) => {
                        last_err = e;
                        continue;
                    }
                };
                let device_id = {
                    let s = as_str(&v, "device_id_str");
                    if s.is_empty() { as_str(&v, "device_id") } else { s }
                };
                let install_id = {
                    let s = as_str(&v, "install_id_str");
                    if s.is_empty() { as_str(&v, "install_id") } else { s }
                };
                if device_id.is_empty() || device_id == "0" || install_id.is_empty() || install_id == "0" {
                    last_err = format!("设备注册未返回有效 ID: {}", String::from_utf8_lossy(&raw));
                    continue;
                }
                return Ok(DeviceIdentity {
                    device_id,
                    install_id,
                    device_token: as_str(&v, "device_token"),
                    cdid,
                    openudid,
                    region: "cn-zj".into(),
                    model: DeviceModel {
                        device_model: profile.device_model.into(),
                        device_brand: profile.device_brand.into(),
                        device_manufacturer: profile.device_manufacturer.into(),
                        os_version: profile.os_version.into(),
                        os_api: profile.os_api,
                        resolution: profile.resolution.into(),
                        dpi: profile.dpi,
                        rom_version: profile.rom_version.into(),
                    },
                    created_at: now_secs(),
                });
            }
            Err(e) => last_err = e,
        }
    }
    Err(format!("设备注册失败: {}", last_err))
}

fn cookie_header() -> Result<String, String> {
    let identity = device_identity()?;
    Ok(format!(
        "store-region={}; store-region-src=did; install_id={}",
        identity.region, identity.install_id
    ))
}

// ===== 数据结构 =====

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BookDetail {
    pub id: String,
    pub title: String,
    pub author: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub cover: String,
    pub category: String,
    pub status: String,
    pub word_count: u64,
    pub read_count: u64,
    pub last_chapter_title: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterItem {
    pub item_id: String,
    pub title: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RankBook {
    pub rank: String,
    pub id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub cover: String,
    pub score: String,
    pub heat: String,
    pub word_count: u64,
    pub status: String,
}

// ===== 编码工具 =====

fn hex_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16).unwrap_or(0) as u8;
        let lo = (bytes[i + 1] as char).to_digit(16).unwrap_or(0) as u8;
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

fn hex_lower(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16).unwrap_or(0) as u8;
            let lo = (bytes[i + 2] as char).to_digit(16).unwrap_or(0) as u8;
            out.push((hi << 4) | lo);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn query_param(query: &str, key: &str) -> String {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return url_decode(v);
            }
        }
    }
    String::new()
}

fn device_query(extra: &[(&str, String)]) -> Result<String, String> {
    let identity = device_identity()?;
    let model = &identity.model;
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in [
        ("iid", identity.install_id.as_str()),
        ("device_id", identity.device_id.as_str()),
        ("cdid", identity.cdid.as_str()),
        ("device_type", model.device_model.as_str()),
        ("device_brand", model.device_brand.as_str()),
        ("os_version", model.os_version.as_str()),
        ("resolution", model.resolution.as_str()),
    ] {
        parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    parts.push(format!("os_api={}", model.os_api));
    parts.push(format!("dpi={}", model.dpi));
    for (k, v) in DEVICE_STATIC {
        parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    for (k, v) in extra {
        parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    Ok(parts.join("&"))
}

// ===== protobuf / 密码学 =====

fn put_varint(mut n: u64, out: &mut Vec<u8>) {
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            out.push(b | 0x80);
        } else {
            out.push(b);
            break;
        }
    }
}

fn field_varint(field: u64, value: u64, out: &mut Vec<u8>) {
    put_varint(field << 3, out);
    put_varint(value, out);
}

fn field_bytes(field: u64, data: &[u8], out: &mut Vec<u8>) {
    put_varint((field << 3) | 2, out);
    put_varint(data.len() as u64, out);
    out.extend_from_slice(data);
}

fn sm3_hash(data: &[u8]) -> [u8; 32] {
    use sm3::Digest as _;
    use sm3::Sm3;
    let mut h = Sm3::new();
    h.update(data);
    let out = h.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&out);
    result
}

fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let n = 16 - data.len() % 16;
    let mut out = data.to_vec();
    out.extend(std::iter::repeat(n as u8).take(n));
    out
}

fn pkcs7_unpad(data: &[u8]) -> Vec<u8> {
    if let Some(&n) = data.last() {
        let n = n as usize;
        if (1..=16).contains(&n) && data.len() >= n && data[data.len() - n..].iter().all(|&b| b == n as u8) {
            return data[..data.len() - n].to_vec();
        }
    }
    data.to_vec()
}

fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES key");
    let padded = pkcs7_pad(data);
    let mut prev = [0u8; 16];
    prev.copy_from_slice(iv);
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = [0u8; 16];
        for i in 0..16 {
            block[i] = chunk[i] ^ prev[i];
        }
        cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
        out.extend_from_slice(&block);
        prev = block;
    }
    out
}

fn aes_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES key");
    let mut prev = [0u8; 16];
    prev.copy_from_slice(iv);
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        if chunk.len() < 16 {
            break;
        }
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        let ct = block;
        cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
        for i in 0..16 {
            block[i] ^= prev[i];
        }
        out.extend_from_slice(&block);
        prev = ct;
    }
    pkcs7_unpad(&out)
}

fn rotl64(x: u64, n: u32) -> u64 {
    x.rotate_left(n)
}

fn rotr64(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

fn simon_keys(k: &[u64; 4]) -> Vec<u64> {
    let mut key = vec![0u64; 72];
    key[..4].copy_from_slice(k);
    for i in 4..72 {
        let mut tmp = rotr64(key[i - 1], 3);
        tmp ^= key[i - 3];
        tmp ^= rotr64(tmp, 1);
        let bit = (0x3DC94C3A046D678B_u64 >> ((i - 4) % 62)) & 1;
        key[i] = (!key[i - 4]) ^ tmp ^ bit ^ 3;
    }
    key
}

fn simon_encrypt(pt: [u64; 2], keys: &[u64]) -> [u64; 2] {
    let mut x = pt[0];
    let mut y = pt[1];
    for k in keys.iter().take(72) {
        let f = rotl64(y, 1) & rotl64(y, 8);
        let next = x ^ f ^ rotl64(y, 2) ^ *k;
        x = y;
        y = next;
    }
    [x, y]
}

fn build_argus(query: &str, ts: i64, rand: u32) -> String {
    use md5::Digest as _;
    let device_id = query_param(query, "device_id");
    let version_name = query_param(query, "version_name");
    let zero = [0u8; 16];
    let body_hash = sm3_hash(&zero);
    let query_hash = sm3_hash(query.as_bytes());

    let mut inner15 = Vec::new();
    field_varint(1, 1, &mut inner15);
    field_varint(2, 1, &mut inner15);
    field_varint(3, 1, &mut inner15);
    field_varint(7, 3348294860, &mut inner15);

    let mut inner23 = Vec::new();
    field_bytes(1, b"NX551J", &mut inner23);
    field_varint(2, 8196, &mut inner23);
    field_varint(4, 2162219008, &mut inner23);

    let mut pb = Vec::new();
    field_varint(1, 0x20200929u64 * 2, &mut pb);
    field_varint(2, 2, &mut pb);
    field_varint(3, rand as u64, &mut pb);
    field_bytes(4, b"1967", &mut pb);
    field_bytes(5, device_id.as_bytes(), &mut pb);
    field_bytes(6, b"1611921764", &mut pb);
    field_bytes(7, version_name.as_bytes(), &mut pb);
    field_bytes(8, b"v04.04.05-ov-android", &mut pb);
    field_varint(9, 134744640, &mut pb);
    field_bytes(10, &[0u8; 8], &mut pb);
    field_varint(11, 0, &mut pb);
    field_varint(12, (ts as u64) * 2, &mut pb);
    field_bytes(13, &body_hash[..6], &mut pb);
    field_bytes(14, &query_hash[..6], &mut pb);
    field_bytes(15, &inner15, &mut pb);
    field_bytes(16, b"", &mut pb);
    field_bytes(20, b"none", &mut pb);
    field_varint(21, 738, &mut pb);
    field_bytes(23, &inner23, &mut pb);
    field_varint(25, 2, &mut pb);
    let pb = pkcs7_pad(&pb);

    let mut sm3_input = Vec::new();
    sm3_input.extend_from_slice(&SIGN_KEY);
    sm3_input.extend_from_slice(&LOW_RAND);
    sm3_input.extend_from_slice(&HIGH_RAND);
    sm3_input.extend_from_slice(&SIGN_KEY);
    let sm3_out = sm3_hash(&sm3_input);
    let mut keys = [0u64; 4];
    for i in 0..4 {
        keys[i] = u64::from_le_bytes(sm3_out[i * 8..i * 8 + 8].try_into().unwrap());
    }
    let simon = simon_keys(&keys);

    let mut enc = Vec::with_capacity(pb.len());
    for chunk in pb.chunks(16) {
        let a = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let b = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        let [lo, hi] = simon_encrypt([a, b], &simon);
        enc.extend_from_slice(&lo.to_le_bytes());
        enc.extend_from_slice(&hi.to_le_bytes());
    }

    let mut data = Vec::with_capacity(8 + enc.len());
    data.extend_from_slice(&XOR_PREFIX);
    data.extend_from_slice(&enc);
    for i in 8..data.len() {
        data[i] ^= data[i % 8];
    }
    data.reverse();

    let mut plain = Vec::new();
    plain.extend_from_slice(&HEADER);
    plain.extend_from_slice(&data);
    plain.extend_from_slice(&HIGH_RAND);

    let key = md5::Md5::digest(&SIGN_KEY[..16]);
    let iv = md5::Md5::digest(&SIGN_KEY[16..]);
    let ct = aes_cbc_encrypt(&key, &iv, &plain);
    let mut out = Vec::with_capacity(2 + ct.len());
    out.extend_from_slice(&LOW_RAND);
    out.extend_from_slice(&ct);
    base64::engine::general_purpose::STANDARD.encode(out)
}

fn speck_keys(key: &[u8; 32]) -> Vec<u64> {
    let key0 = u64::from_le_bytes(key[0..8].try_into().unwrap());
    let mut ls = vec![
        u64::from_le_bytes(key[8..16].try_into().unwrap()),
        u64::from_le_bytes(key[16..24].try_into().unwrap()),
        u64::from_le_bytes(key[24..32].try_into().unwrap()),
    ];
    let mut ks = vec![key0];
    for x in 0..33 {
        let rs = rotr64(ls[x], 8);
        let new_x = (x as u64) ^ rs.wrapping_add(ks[x]);
        let ls_y = rotl64(ks[x], 3);
        ls.push(new_x);
        ks.push(new_x ^ ls_y);
    }
    ks
}

fn speck_encrypt(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    let padded = pkcs7_pad(data);
    let ks = speck_keys(key);
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut y = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let mut x = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        for k in &ks {
            let rs = rotr64(x, 8);
            x = *k ^ rs.wrapping_add(y);
            y = x ^ rotl64(y, 3);
        }
        out.extend_from_slice(&y.to_le_bytes());
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn build_ladon(ts: i64, rand: [u8; 4]) -> String {
    let plaintext = format!("{}-1611921764-1967", ts);
    let mut md5 = md5::Md5::new();
    use md5::Digest as _;
    md5.update(rand);
    md5.update(b"1967");
    let key_hex = hex_lower(&md5.finalize());
    let key: [u8; 32] = key_hex.as_bytes().try_into().unwrap();
    let ct = speck_encrypt(&key, plaintext.as_bytes());
    let mut out = Vec::with_capacity(4 + ct.len());
    out.extend_from_slice(&rand);
    out.extend_from_slice(&ct);
    base64::engine::general_purpose::STANDARD.encode(out)
}

/// 生成 X-Argus / X-Ladon / X-Khronos。
pub fn sign_headers(query: &str, ts: i64) -> HashMap<String, String> {
    let mut rng = rand::thread_rng();
    let mut rand4 = [0u8; 4];
    rng.fill_bytes(&mut rand4);
    let argus_rand = u32::from_le_bytes(rand4) % 0x7FFF_FFFF;

    let mut h = HashMap::new();
    h.insert("X-Argus".into(), build_argus(query, ts, argus_rand));
    h.insert("X-Ladon".into(), build_ladon(ts, rand4));
    h.insert("X-Khronos".into(), ts.to_string());
    h
}

// ===== HTTP =====

static HTTP_AGENT: OnceLock<ureq::Agent> = OnceLock::new();

fn http_agent() -> &'static ureq::Agent {
    HTTP_AGENT.get_or_init(|| {
        let tls = ureq::native_tls::TlsConnector::new().expect("初始化 TLS 失败");
        ureq::AgentBuilder::new()
            .tls_connector(Arc::new(tls))
            .timeout(Duration::from_secs(40))
            .build()
    })
}

fn http(method: &str, url: &str, headers: &HashMap<String, String>, body: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let agent = http_agent();
    let mut req = if method == "POST" { agent.post(url) } else { agent.get(url) };
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = if let Some(b) = body {
        req.send_bytes(b)
    } else {
        req.call()
    };
    let resp = resp.map_err(|e| format!("请求失败: {}", e))?;
    let mut reader = resp.into_reader();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).map_err(|e| format!("读取响应失败: {}", e))?;
    Ok(buf)
}

fn web_headers() -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("User-Agent".into(), UA_WEB.into());
    h.insert("Accept".into(), "application/json".into());
    h
}

fn app_headers(query: &str, ts: i64) -> Result<HashMap<String, String>, String> {
    let mut h = HashMap::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut rng = rand::thread_rng();
    let mut r2 = [0u8; 2];
    rng.fill_bytes(&mut r2);
    let reading = format!("{}-{}", now, u16::from_be_bytes(r2));
    h.insert("User-Agent".into(), UA_APP.into());
    h.insert("Cookie".into(), cookie_header()?);
    h.insert("Accept".into(), "application/json; charset=utf-8,application/x-protobuf".into());
    h.insert("Accept-Encoding".into(), "gzip".into());
    h.insert("x-xs-from-web".into(), "0".into());
    h.insert("x-ss-req-ticket".into(), now.to_string());
    h.insert("x-reading-request".into(), reading);
    h.insert("x-vc-bdturing-sdk-version".into(), "3.7.2.cn".into());
    h.insert("lc".into(), "101".into());
    h.insert("sdk-version".into(), "2".into());
    h.insert("passport-sdk-version".into(), "50564".into());
    h.insert("x-tt-store-region".into(), "cn-zj".into());
    h.insert("x-tt-store-region-src".into(), "did".into());
    for (k, v) in sign_headers(query, ts) {
        h.insert(k, v);
    }
    Ok(h)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn json_value(bytes: &[u8]) -> Result<serde_json::Value, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("响应解析失败: {}", e))
}

fn as_str(v: &serde_json::Value, key: &str) -> String {
    match v.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn as_u64(v: &serde_json::Value, key: &str) -> u64 {
    v.get(key)
        .and_then(|x| {
            x.as_u64()
                .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0)
}

/// categoryV2 有时是 JSON 数组字符串，取其中的 Name。
fn category_name(d: &serde_json::Value) -> String {
    let raw = as_str(d, "categoryV2");
    if raw.starts_with('[') {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            let names: Vec<String> = arr
                .iter()
                .filter_map(|x| x.get("Name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect();
            if !names.is_empty() {
                return names.join(" / ");
            }
        }
    }
    let c = as_str(d, "category");
    if c.is_empty() { raw } else { c }
}

// ===== 免签接口 =====

pub fn book_info(book_id: &str) -> Result<BookDetail, String> {
    let url = format!("{}/api/book/info?bookId={}", WEB_BASE, url_encode(book_id));
    let body = http("GET", &url, &web_headers(), None)?;
    let v = json_value(&body)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("书籍详情接口错误: code={}", as_str(&v, "code")));
    }
    let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let cover = {
        let u = as_str(&d, "thumbUrl");
        if u.is_empty() { as_str(&d, "thumbUri") } else { u }
    };
    Ok(BookDetail {
        id: as_str(&d, "bookId"),
        title: as_str(&d, "bookName"),
        author: as_str(&d, "author"),
        abstract_text: as_str(&d, "abstract"),
        cover,
        category: category_name(&d),
        status: as_str(&d, "creationStatus"),
        word_count: as_u64(&d, "wordNumber"),
        read_count: as_u64(&d, "readCount"),
        last_chapter_title: as_str(&d, "lastChapterTitle"),
    })
}

pub fn chapters(book_id: &str) -> Result<Vec<ChapterItem>, String> {
    let url = format!("{}/api/reader/directory/detail?bookId={}", WEB_BASE, url_encode(book_id));
    let body = http("GET", &url, &web_headers(), None)?;
    let v = json_value(&body)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("目录接口错误: code={}", as_str(&v, "code")));
    }
    let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let mut titles: HashMap<String, String> = HashMap::new();
    if let Some(vols) = d.get("chapterListWithVolume").and_then(|x| x.as_array()) {
        for vol in vols {
            if let Some(chs) = vol.as_array() {
                for ch in chs {
                    let id = as_str(ch, "itemId");
                    if !id.is_empty() {
                        titles.insert(id, as_str(ch, "title"));
                    }
                }
            }
        }
    }
    let ids = d
        .get("allItemIds")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(ids
        .iter()
        .filter_map(|id| id.as_str())
        .map(|id| ChapterItem {
            item_id: id.to_string(),
            title: titles.get(id).cloned().unwrap_or_default(),
        })
        .collect())
}

// ===== 签发接口（正文） =====

static CONTENT_KEY: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();

pub fn content_key(force: bool) -> Result<String, String> {
    let cell = CONTENT_KEY.get_or_init(|| Mutex::new(None));
    {
        let cache = cell.lock().map_err(|_| "密钥缓存锁失败".to_string())?;
        if !force {
            if let Some((key, at)) = cache.as_ref() {
                if at.elapsed() < Duration::from_secs(1800) {
                    return Ok(key.clone());
                }
            }
        }
    }

    let ts = now_secs();
    let query = device_query(&[("_rticket", now_millis().to_string())])?;
    let url = format!("{}/reading/crypt/registerkey?{}", APP_BASE, query);

    let device_id: u64 = device_identity()?.device_id.parse().unwrap_or(0);
    let mut combined = Vec::with_capacity(16);
    combined.extend_from_slice(&device_id.to_le_bytes());
    combined.extend_from_slice(&[0u8; 8]);
    let mut iv = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut iv);
    let blob = aes_cbc_encrypt(&hex_decode(REG_KEY), &iv, &combined);
    let mut payload = iv.to_vec();
    payload.extend_from_slice(&blob);
    let body = serde_json::json!({
        "content": base64::engine::general_purpose::STANDARD.encode(payload),
        "keyver": 1,
    })
    .to_string();

    let mut headers = app_headers(&query, ts)?;
    headers.insert("Content-Type".into(), "application/json".into());
    let raw = http("POST", &url, &headers, Some(body.as_bytes()))?;
    let v = json_value(&raw)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("registerkey 错误: code={} {}", as_str(&v, "code"), as_str(&v, "message")));
    }
    let enc = as_str(&v, "data");
    let key_b64 = v
        .pointer("/data/key")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "registerkey 未返回密钥".to_string())?;
    let _ = enc;
    let raw_key = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .map_err(|e| format!("密钥解码失败: {}", e))?;
    if raw_key.len() < 32 {
        return Err("registerkey 密钥长度异常".into());
    }
    let dec = aes_cbc_decrypt(&hex_decode(REG_KEY), &raw_key[..16], &raw_key[16..]);
    let real = hex_lower(&dec).to_uppercase();
    let real = real.chars().take(32).collect::<String>();
    let mut cache = cell.lock().map_err(|_| "密钥缓存锁失败".to_string())?;
    *cache = Some((real.clone(), Instant::now()));
    Ok(real)
}

/// App 榜单：bookmall/tab 的排行榜 tab。algo：101 推荐、100 完本、200 巅峰、104 口碑。
pub fn rank_books(algo: &str) -> Result<Vec<RankBook>, String> {
    let now_ms = now_millis().to_string();
    let query = device_query(&[
        ("_rticket", now_ms.clone()),
        ("tab_type", "23".to_string()),
        ("tab_index", "0".to_string()),
        ("req_rank_algo", algo.to_string()),
        ("req_rank_category_id", "0".to_string()),
    ])?;
    let url = format!("{}/reading/bookapi/bookmall/tab/v?{}", APP_BASE, query);
    let headers = app_headers(&query, now_secs())?;
    let raw = http("GET", &url, &headers, None)?;
    let v = json_value(&raw)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("榜单接口错误: code={} {}", as_str(&v, "code"), as_str(&v, "message")));
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let tabs = v
        .get("data")
        .and_then(|d| d.get("tab_item"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    for tab in tabs {
        let cells = tab.get("cell_data").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        for cell in cells {
            if cell.get("show_type").and_then(|x| x.as_i64()) != Some(511) {
                continue;
            }
            let entries = cell.get("cell_data").and_then(|x| x.as_array()).cloned().unwrap_or_default();
            for entry in entries {
                let books = match entry.get("book_data") {
                    Some(serde_json::Value::Array(a)) => a.clone(),
                    Some(b @ serde_json::Value::Object(_)) => vec![b.clone()],
                    _ => Vec::new(),
                };
                for b in books {
                    let id = as_str(&b, "book_id");
                    if id.is_empty() || !seen.insert(id.clone()) {
                        continue;
                    }
                    let rank = serde_json::from_str::<serde_json::Value>(&as_str(&b, "recommend_info"))
                        .ok()
                        .and_then(|x| x.get("rank").cloned())
                        .map(|r| match r {
                            serde_json::Value::String(s) => s,
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    let mut cover = as_str(&b, "thumb_url");
                    if cover.is_empty() {
                        let audio = as_str(&b, "audio_thumb_uri");
                        let horizontal = as_str(&b, "horiz_thumb_url");
                        if !audio.is_empty() {
                            cover = audio;
                        } else if !horizontal.is_empty() {
                            cover = horizontal;
                        }
                    }
                    out.push(RankBook {
                        rank,
                        id,
                        title: as_str(&b, "book_name"),
                        author: as_str(&b, "author"),
                        category: as_str(&b, "category"),
                        abstract_text: as_str(&b, "abstract"),
                        cover,
                        score: as_str(&b, "score"),
                        heat: as_str(&b, "rank_score"),
                        word_count: as_u64(&b, "word_number"),
                        status: as_str(&b, "creation_status"),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// App 搜索：/reading/bookapi/search/page/v，返回真实评分。
pub fn search_app(query: &str) -> Result<Vec<crate::shared::StoreBook>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let query = device_query(&[
        ("_rticket", now_millis().to_string()),
        ("query", q.to_string()),
        ("offset", "0".to_string()),
        ("count", "10".to_string()),
    ])?;
    let url = format!("{}/reading/bookapi/search/page/v?{}", APP_BASE, query);
    let headers = app_headers(&query, now_secs())?;
    let raw = http("GET", &url, &headers, None)?;
    let v = json_value(&raw)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("搜索接口错误: code={} {}", as_str(&v, "code"), as_str(&v, "message")));
    }
    let mut out = Vec::new();
    let cells = v.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();
    for cell in cells {
        let book = match cell.get("book_data") {
            Some(serde_json::Value::Array(a)) => a.first().cloned().unwrap_or_default(),
            Some(b @ serde_json::Value::Object(_)) => b.clone(),
            _ => continue,
        };
        let id = as_str(&cell, "book_id");
        let id = if id.is_empty() { as_str(&book, "book_id") } else { id };
        if id.is_empty() {
            continue;
        }
        let audio = as_str(&book, "audio_thumb_uri");
        let thumb = as_str(&book, "thumb_url");
        let title = {
            let t = as_str(&book, "book_name");
            if t.is_empty() { as_str(&cell, "cell_name") } else { t }
        };
        out.push(crate::shared::StoreBook {
            id,
            title,
            author: as_str(&book, "author"),
            abstract_text: as_str(&book, "abstract"),
            cover: if thumb.is_empty() { audio } else { thumb },
            score: as_str(&book, "score"),
            category: as_str(&book, "category"),
        });
    }
    Ok(out)
}

static COVER_CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn cover_host_allowed(url: &str) -> bool {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else { return false };
    let host = rest.split('/').next().unwrap_or("").split(':').next().unwrap_or("");
    host.ends_with("fqnovelpic.com")
        || host.ends_with("byteimg.com")
        || host.ends_with("bytecdn.cn")
}

/// 下载 HEIC 并解码为 JPEG；只允许番茄图片 CDN，带进程内缓存。
pub fn cover_jpeg(url: &str) -> Result<Vec<u8>, String> {
    if !cover_host_allowed(url) {
        return Err("不支持的封面地址".into());
    }
    let cache = COVER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(bytes) = cache.lock().map_err(|e| e.to_string())?.get(url).cloned() {
        return Ok(bytes);
    }
    let raw = http("GET", url, &web_headers(), None)?;
    let jpeg = crate::heif::decode_heic(&raw)?;
    let mut guard = cache.lock().map_err(|e| e.to_string())?;
    if guard.len() > 256 {
        guard.clear();
    }
    guard.insert(url.to_string(), jpeg.clone());
    Ok(jpeg)
}

/// 批量取封面；最多 4 路并发，顺序与输入一致，失败项为 None。
pub fn cover_batch(urls: &[String]) -> Vec<Option<Vec<u8>>> {
    let out = Arc::new(Mutex::new(vec![None; urls.len()]));
    let next = Arc::new(AtomicUsize::new(0));
    let workers = urls.len().min(4);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let out = Arc::clone(&out);
            let next = Arc::clone(&next);
            scope.spawn(move || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= urls.len() {
                    break;
                }
                if let Ok(bytes) = cover_jpeg(&urls[index]) {
                    if let Ok(mut guard) = out.lock() {
                        guard[index] = Some(bytes);
                    }
                }
            });
        }
    });
    Arc::try_unwrap(out)
        .ok()
        .and_then(|m| m.into_inner().ok())
        .unwrap_or_default()
}

pub fn fetch_contents(item_ids: &[String], book_id: &str) -> Result<HashMap<String, String>, String> {
    fetch_contents_mode(item_ids, book_id, "1")
}

/// 下载模式：req_type=0（Download），批量返回完整正文，适合离线导出。
pub fn fetch_contents_download(item_ids: &[String], book_id: &str) -> Result<HashMap<String, String>, String> {
    fetch_contents_mode(item_ids, book_id, "0")
}

fn fetch_contents_mode(item_ids: &[String], book_id: &str, req_type: &str) -> Result<HashMap<String, String>, String> {
    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let real = content_key(false)?;
    let key_bytes = hex_decode(&real);
    let ts = now_secs();
    let now_ms = now_millis().to_string();
    let query = device_query(&[
        ("_rticket", now_ms.clone()),
        ("item_ids", item_ids.join(",")),
        ("book_id", book_id.to_string()),
        ("req_type", req_type.to_string()),
        ("key_register_ts", now_ms),
    ])?;
    let url = format!("{}/reading/reader/batch_full/v?{}", APP_BASE, query);
    let headers = app_headers(&query, ts)?;
    let raw = http("GET", &url, &headers, None)?;
    let v = json_value(&raw)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("正文接口错误: code={} {}", as_str(&v, "code"), as_str(&v, "message")));
    }
    let mut out = HashMap::new();
    if let Some(data) = v.get("data").and_then(|x| x.as_object()) {
        for (iid, item) in data {
            let content = item.get("content").and_then(|x| x.as_str()).unwrap_or("");
            if content.is_empty() {
                continue;
            }
            let ct = match base64::engine::general_purpose::STANDARD.decode(content) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if ct.len() < 32 {
                continue;
            }
            let mut plain = aes_cbc_decrypt(&key_bytes, &ct[..16], &ct[16..]);
            if plain.len() >= 2 && plain[0] == 0x1f && plain[1] == 0x8b {
                use flate2::read::GzDecoder;
                let mut d = GzDecoder::new(&plain[..]);
                let mut buf = Vec::new();
                if d.read_to_end(&mut buf).is_ok() {
                    plain = buf;
                }
            }
            let html = String::from_utf8_lossy(&plain).to_string();
            out.insert(iid.clone(), html_to_text(&html));
        }
    }
    Ok(out)
}

fn html_to_text(html: &str) -> String {
    static HEAD: OnceLock<regex::Regex> = OnceLock::new();
    static STYLE: OnceLock<regex::Regex> = OnceLock::new();
    static SCRIPT: OnceLock<regex::Regex> = OnceLock::new();
    static TAG: OnceLock<regex::Regex> = OnceLock::new();
    static MULTI: OnceLock<regex::Regex> = OnceLock::new();
    let head = HEAD.get_or_init(|| regex::Regex::new(r"(?is)<head\b[^>]*>.*?</head>").unwrap());
    let style = STYLE.get_or_init(|| regex::Regex::new(r"(?is)<style\b[^>]*>.*?</style>").unwrap());
    let script = SCRIPT.get_or_init(|| regex::Regex::new(r"(?is)<script\b[^>]*>.*?</script>").unwrap());
    let tag = TAG.get_or_init(|| regex::Regex::new(r"<[^>]+>").unwrap());
    let multi = MULTI.get_or_init(|| regex::Regex::new(r"\n{2,}").unwrap());
    // 先丢弃 head/style/script 的内部文本，避免 <title>Unknown</title>
    // 和 EPUB 自带的版权页 CSS 被当成正文。
    let cleaned = head.replace_all(html, "");
    let cleaned = style.replace_all(&cleaned, "");
    let cleaned = script.replace_all(&cleaned, "");
    let text = tag.replace_all(&cleaned, "\n");
    multi.replace_all(&text, "\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_skips_head_style_and_script() {
        let html = r#"<html><head><title>Unknown</title><style>.copyright-title { color: red; }</style></head><body><h1>总序</h1><p>正文内容</p><script>alert(1)</script></body></html>"#;
        assert_eq!(html_to_text(html), "总序\n正文内容");
    }

    fn fixed_rand() -> ([u8; 4], u32) {
        ([1, 2, 3, 4], u32::from_le_bytes([1, 2, 3, 4]) % 0x7FFF_FFFF)
    }

    #[test]
    fn sign_headers_matches_reference() {
        let query = "aid=1967&device_id=1694818514274715&iid=1694818514278811&version_name=6.8.1.32";
        let (rand, argus_rand) = fixed_rand();
        assert_eq!(
            build_argus(query, 1700000000, argus_rand),
            "8oGwZRLe2V38m+1gfqIeLa2MoV9ZBWosW5Y0NXKHAB2aEX8Sy3XkKahnaeRieievkKKzQJDA7sUwleXkOtT6ltPx/p/Iw9mdN/sFUci0kmESZgMOMnkKO5yU8TY774xAr7Eg4zc2eGNJTfai9TZB6tU14OTe4n85gVpdWQ9m/QdWJMUn+yG+X2tK4p3aG75fcmPIwUCvTQ8bS81gFNwRQbd5yPVAgp4hgN4rge2mj9zfCfrOMHd5xbUiLsQx/OLsyC2Dr9FDyYkOGHqptIKuO/SN"
        );
        assert_eq!(
            build_ladon(1700000000, rand),
            "AQIDBKM7Y7bWnOIUXmTybD6/MpGbuge4sBwUJsyHVivhtMsH"
        );
    }
}
