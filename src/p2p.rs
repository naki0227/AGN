//! AGN P2P Communication Layer - 近接検出モジュール
//! Eeyo: BLE/Wi-Fi Awareによるリアルタイム位置検出
//!
//! Phase 13: 「ええよ」SNSのための空間通信基盤

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ed25519_dalek::{Signer, Verifier, VerifyingKey, Signature};
use rand::Rng;

/// ビーコンタイプ（ユーザーの状態を表す）
#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub enum BeaconType {
    /// 助けを求めている
    NeedHelp,
    /// 暇（助けられる状態）
    Idle,
    /// 観光中
    Touring,
    /// カスタム状態
    Custom(u8),
}

impl BeaconType {
    /// ビーコンタイプを1バイトにエンコード
    pub fn to_byte(&self) -> u8 {
        match self {
            BeaconType::NeedHelp => 0x01,
            BeaconType::Idle => 0x02,
            BeaconType::Touring => 0x03,
            BeaconType::Custom(v) => 0x80 | (*v & 0x7F),
        }
    }

    /// 1バイトからビーコンタイプをデコード
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x01 => BeaconType::NeedHelp,
            0x02 => BeaconType::Idle,
            0x03 => BeaconType::Touring,
            v => BeaconType::Custom(v & 0x7F),
        }
    }
}

// ============================================================
// ビーコンパケット設計 (BLE Advertising Data Format)
// ============================================================

/// Eeyoビーコンパケット（31バイト以内のBLE制限に準拠）
/// 
/// ```text
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | Magic  | Ver    | Type   | Toku   | Toku   | UserID | UserID | UserID |
/// | (0xEE) | (0x01) | (1B)   | Hi(1B) | Lo(1B) | [0]    | [1]    | [2]    |
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | UserID | UserID | UserID | UserID | UserID | Flags  | Lat    | Lat    |
/// | [3]    | [4]    | [5]    | [6]    | [7]    | (1B)   | Hi(1B) | Lo(1B) |
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | Lon    | Lon    | TTL    | CRC    |
/// | Hi(1B) | Lo(1B) | (1B)   | (1B)   |
/// +--------+--------+--------+--------+
/// ```
/// 
/// 合計: 20バイト（BLE Advertising 31バイト制限内）
#[derive(Debug, Clone)]
pub struct EeyoBeaconPacket {
    /// マジックバイト (0xEE = "ええよ")
    pub magic: u8,
    /// プロトコルバージョン
    pub version: u8,
    /// ビーコンタイプ
    pub beacon_type: BeaconType,
    /// 徳スコア (0-65535)
    pub toku_score: u16,
    /// ユーザーID (8バイトハッシュ)
    pub user_id: [u8; 8],
    /// フラグ
    /// - bit 0: 位置情報あり
    /// - bit 1: 言語対応 (0=日本語, 1=英語)
    /// - bit 2-7: 予約
    pub flags: u8,
    /// 緯度 (オプション、精度約0.01度)
    pub latitude: Option<i16>,
    /// 経度 (オプション、精度約0.01度)
    pub longitude: Option<i16>,
    /// TTL (秒単位、最大255秒)
    pub ttl: u8,
}

/// セキュアなビーコンパケット (Phase 17)
/// 
/// ```text
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | Magic  | Ver    | Type   | Toku   | Toku   | Nonce  | Nonce  | Nonce  |
/// | (0xEE) | (0x02) | (1B)   | Hi(1B) | Lo(1B) | [0]    | [1]    | [2]    |
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | Nonce  | Time   | Time   | Time   | Time   | Time   | Time   | Time   |
/// | [3]    | [0]    | [1]    | [2]    | [3]    | [4]    | [5]    | [6]    |
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// | Time   | PubKey | ...    | PubKey | Sig    | ...    | Sig    |
/// | [7]    | (32B)  |        | [31]   | (64B)  |        | [63]   |
/// +--------+--------+--------+--------+--------+--------+--------+--------+
/// ```
/// Total: ~112 bytes
#[derive(Debug, Clone)]
pub struct EeyoSecurePacket {
    /// マジックバイト (0xEE)
    pub magic: u8,
    /// プロトコルバージョン (0x02)
    pub version: u8,
    /// ビーコンタイプ
    pub beacon_type: BeaconType,
    /// 徳スコア
    pub toku_score: u16,
    /// ナンス (重複排除用)
    pub nonce: u32,
    /// タイムスタンプ (UNIX Epoch ms)
    pub timestamp: u64,
    /// 送信者公開鍵 (32 bytes)
    pub sender_public_key: [u8; 32],
    /// Ed25519署名 (64 bytes)
    pub signature: [u8; 64],
}

impl EeyoSecurePacket {
    pub const MAGIC: u8 = 0xEE;
    pub const VERSION: u8 = 0x02;
    pub const PACKET_SIZE: usize = 113; // 49 (Payload) + 64 (Sig)

    pub fn new(
        beacon_type: BeaconType,
        toku_score: u16,
        public_key: &[u8; 32],
        secret_key: &ed25519_dalek::SigningKey,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let nonce = rng.gen::<u32>();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut packet = Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            beacon_type,
            toku_score,
            nonce,
            timestamp,
            sender_public_key: *public_key,
            signature: [0u8; 64],
        };

        packet.sign(secret_key);
        packet
    }

    /// 署名対象のバイト列を取得
    fn to_bytes_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(48);
        bytes.push(self.magic);
        bytes.push(self.version);
        bytes.push(self.beacon_type.to_byte()); // Corrected from `self.beacon_type as u8`
        bytes.extend_from_slice(&self.toku_score.to_be_bytes());
        bytes.extend_from_slice(&self.nonce.to_be_bytes());
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes.extend_from_slice(&self.sender_public_key);
        bytes
    }

    /// 署名を生成して設定
    pub fn sign(&mut self, secret_key: &ed25519_dalek::SigningKey) {
        let bytes = self.to_bytes_for_signing();
        self.signature = secret_key.sign(&bytes).to_bytes();
    }

    /// 署名とタイムスタンプを検証
    pub fn verify(&self) -> bool {
        // 1. Timestamp Check (Allow +/- 30 seconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        if self.timestamp > now + 30_000 || self.timestamp < now.saturating_sub(30_000) {
            // log::warn!("[Security] Timestamp out of range: {} (now: {})", self.timestamp, now); // Commented out as `log` crate is not imported
            return false;
        }

        // 2. Signature Check
        let verifying_key = match VerifyingKey::from_bytes(&self.sender_public_key) {
            Ok(vk) => vk,
            Err(_) => return false,
        };
        
        let signature = Signature::from_bytes(&self.signature);
        let bytes = self.to_bytes_for_signing();
        
        verifying_key.verify(&bytes, &signature).is_ok()
    }

    /// フルパケットをバイト列にシリアライズ
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.to_bytes_for_signing();
        bytes.extend_from_slice(&self.signature);
        bytes
    }

    /// バイト列からパケットを復元し、検証を行う
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::PACKET_SIZE {
            return None;
        }
        
        if bytes[0] != Self::MAGIC || bytes[1] != Self::VERSION {
            return None;
        }
        
        let beacon_type = BeaconType::from_byte(bytes[2]);
        let toku_score = u16::from_be_bytes([bytes[3], bytes[4]]);
        let nonce = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let timestamp = u64::from_be_bytes([
            bytes[9], bytes[10], bytes[11], bytes[12],
            bytes[13], bytes[14], bytes[15], bytes[16],
        ]);
        
        let mut sender_public_key = [0u8; 32];
        sender_public_key.copy_from_slice(&bytes[17..49]);
        
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[49..113]);
        
        let packet = Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            beacon_type,
            toku_score,
            nonce,
            timestamp,
            sender_public_key,
            signature,
        };
        
        if packet.verify() {
            Some(packet)
        } else {
            None
        }
    }
}

/// セキュリティコンテキスト (鍵管理)
pub struct SecurityContext {
    pub signing_key: ed25519_dalek::SigningKey,
    pub verifying_key: ed25519_dalek::VerifyingKey,
}

impl SecurityContext {
    /// 新しいキーペアを生成
    pub fn new() -> Self {
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut key_bytes);
        
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// バイト列からキーペアを復元 (永続化用)
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// 署名鍵をバイト列として取得 (永続化用)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

pub static SECURITY_CONTEXT: once_cell::sync::Lazy<std::sync::Mutex<SecurityContext>> = once_cell::sync::Lazy::new(|| {
    std::sync::Mutex::new(SecurityContext::new())
});

impl EeyoBeaconPacket {
    /// パケットサイズ (バイト)
    pub const PACKET_SIZE: usize = 20;
    /// マジックバイト
    pub const MAGIC: u8 = 0xEE;
    /// 現在のプロトコルバージョン
    pub const VERSION: u8 = 0x01;

    /// 新しいビーコンパケットを作成
    pub fn new(beacon_type: BeaconType, toku_score: u16, user_id: [u8; 8]) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            beacon_type,
            toku_score,
            user_id,
            flags: 0,
            latitude: None,
            longitude: None,
            ttl: 30, // デフォルト30秒
        }
    }

    /// 位置情報を設定
    pub fn with_location(mut self, lat: f64, lon: f64) -> Self {
        // 緯度・経度を100倍して整数に変換（精度約0.01度 ≈ 1km）
        self.latitude = Some((lat * 100.0) as i16);
        self.longitude = Some((lon * 100.0) as i16);
        self.flags |= 0x01; // 位置情報フラグ
        self
    }

    /// バイト列にシリアライズ
    pub fn to_bytes(&self) -> [u8; Self::PACKET_SIZE] {
        let mut bytes = [0u8; Self::PACKET_SIZE];
        
        bytes[0] = self.magic;
        bytes[1] = self.version;
        bytes[2] = self.beacon_type.to_byte();
        bytes[3] = (self.toku_score >> 8) as u8;  // Toku Hi
        bytes[4] = (self.toku_score & 0xFF) as u8; // Toku Lo
        bytes[5..13].copy_from_slice(&self.user_id);
        bytes[13] = self.flags;
        
        if let Some(lat) = self.latitude {
            bytes[14] = (lat >> 8) as u8;
            bytes[15] = (lat & 0xFF) as u8;
        }
        if let Some(lon) = self.longitude {
            bytes[16] = (lon >> 8) as u8;
            bytes[17] = (lon & 0xFF) as u8;
        }
        
        bytes[18] = self.ttl;
        bytes[19] = self.calculate_crc();
        
        bytes
    }

    /// バイト列からデシリアライズ
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < Self::PACKET_SIZE {
            return Err(format!("パケットサイズ不足: {} < {}", bytes.len(), Self::PACKET_SIZE));
        }
        
        if bytes[0] != Self::MAGIC {
            return Err(format!("不正なマジックバイト: 0x{:02X}", bytes[0]));
        }
        
        let flags = bytes[13];
        let has_location = (flags & 0x01) != 0;
        
        let latitude = if has_location {
            Some(((bytes[14] as i16) << 8) | (bytes[15] as i16))
        } else {
            None
        };
        
        let longitude = if has_location {
            Some(((bytes[16] as i16) << 8) | (bytes[17] as i16))
        } else {
            None
        };
        
        let mut user_id = [0u8; 8];
        user_id.copy_from_slice(&bytes[5..13]);
        
        let packet = Self {
            magic: bytes[0],
            version: bytes[1],
            beacon_type: BeaconType::from_byte(bytes[2]),
            toku_score: ((bytes[3] as u16) << 8) | (bytes[4] as u16),
            user_id,
            flags,
            latitude,
            longitude,
            ttl: bytes[18],
        };
        
        // CRC検証
        let expected_crc = packet.calculate_crc();
        if bytes[19] != expected_crc {
            return Err(format!("CRCエラー: expected 0x{:02X}, got 0x{:02X}", expected_crc, bytes[19]));
        }
        
        Ok(packet)
    }

    /// 簡易CRC計算（XOR チェックサム）
    fn calculate_crc(&self) -> u8 {
        let bytes = self.to_bytes_without_crc();
        bytes.iter().fold(0u8, |acc, &b| acc ^ b)
    }

    fn to_bytes_without_crc(&self) -> [u8; 19] {
        let mut bytes = [0u8; 19];
        bytes[0] = self.magic;
        bytes[1] = self.version;
        bytes[2] = self.beacon_type.to_byte();
        bytes[3] = (self.toku_score >> 8) as u8;
        bytes[4] = (self.toku_score & 0xFF) as u8;
        bytes[5..13].copy_from_slice(&self.user_id);
        bytes[13] = self.flags;
        if let Some(lat) = self.latitude {
            bytes[14] = (lat >> 8) as u8;
            bytes[15] = (lat & 0xFF) as u8;
        }
        if let Some(lon) = self.longitude {
            bytes[16] = (lon >> 8) as u8;
            bytes[17] = (lon & 0xFF) as u8;
        }
        bytes[18] = self.ttl;
        bytes
    }
}

// ============================================================
// 徳スコアマネージャ
// ============================================================

/// 徳スコアの変更理由
#[derive(Debug, Clone)]
pub enum TokuReason {
    /// 助けを提供した
    HelpProvided,
    /// 感謝された
    Thanked,
    /// 推薦された
    Recommended,
    /// ペナルティ
    Penalty,
    /// 初期値
    Initial,
}

/// 徳スコアイベント
#[derive(Debug, Clone)]
pub struct TokuEvent {
    /// 対象ユーザーID
    pub user_id: String,
    /// 変更量（正または負）
    pub delta: i32,
    /// 理由
    pub reason: TokuReason,
    /// タイムスタンプ（Unix秒）
    pub timestamp: u64,
}

/// ユーザー間の関係性（絆 / Bond）
/// 
/// 「ええよ」における継続的な関係を表現
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// 絆レベル (0 = 初対面, 1+ = 絆あり)
    pub level: u32,
    /// 絆の強さ (0-1000) - レベル内での親密度
    pub strength: u32,
    /// 初めて出会った日時 (Unix秒)
    pub first_met: u64,
    /// 最後の接触（Unix秒）
    pub last_interaction: u64,
    /// 助け合い回数
    pub help_count: u32,
    /// 関係タグ (「知人」「親友」等)
    pub tags: Vec<String>,
}

impl Default for Relationship {
    fn default() -> Self {
        Self {
            level: 0,
            strength: 0,
            first_met: 0,
            last_interaction: 0,
            help_count: 0,
            tags: Vec::new(),
        }
    }
}

impl Relationship {
    /// 新しい絆を作成
    pub fn new_bond() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            level: 1,
            strength: 10,
            first_met: now,
            last_interaction: now,
            help_count: 0,
            tags: Vec::new(),
        }
    }

    /// 絆を深める (助け合い成功時)
    pub fn deepen(&mut self, amount: u32) {
        self.help_count += 1;
        self.strength = (self.strength + amount).min(1000);
        self.last_interaction = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // レベルアップ判定 (strength 100 ごとにレベル+1)
        let new_level = (self.strength / 100).max(1);
        if new_level > self.level {
            self.level = new_level;
        }
    }

    /// 絆があるかどうか
    pub fn has_bond(&self) -> bool {
        self.level > 0
    }
}

/// 徳スコアマネージャ
pub struct TokuManager {
    /// ユーザーごとの徳スコア
    scores: Arc<Mutex<HashMap<String, u32>>>,
    /// ユーザー間の関係性 (From -> To)
    relationships: Arc<Mutex<HashMap<(String, String), Relationship>>>,
    /// イベント履歴
    events: Arc<Mutex<Vec<TokuEvent>>>,
}

impl TokuManager {
    /// 初期徳スコア
    pub const INITIAL_SCORE: u32 = 100;
    /// 最大徳スコア
    pub const MAX_SCORE: u32 = 65535;
    
    pub fn new() -> Self {
        Self {
            scores: Arc::new(Mutex::new(HashMap::new())),
            relationships: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 徳スコアを取得（未登録なら初期値）
    pub fn get_score(&self, user_id: &str) -> u32 {
        let scores = self.scores.lock().unwrap();
        *scores.get(user_id).unwrap_or(&Self::INITIAL_SCORE)
    }

    /// 徳スコアを加算
    pub fn add_toku(&self, user_id: &str, amount: u32, reason: TokuReason) {
        let mut scores = self.scores.lock().unwrap();
        let current = *scores.get(user_id).unwrap_or(&Self::INITIAL_SCORE);
        let new_score = (current + amount).min(Self::MAX_SCORE);
        scores.insert(user_id.to_string(), new_score);
        
        // イベント記録
        let mut events = self.events.lock().unwrap();
        events.push(TokuEvent {
            user_id: user_id.to_string(),
            delta: amount as i32,
            reason,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        
        log::info!("[Toku] {} の徳スコア: {} → {}", user_id, current, new_score);
    }

    /// 徳スコアを減算（ペナルティ）
    pub fn subtract_toku(&self, user_id: &str, amount: u32, reason: TokuReason) {
        let mut scores = self.scores.lock().unwrap();
        let current = *scores.get(user_id).unwrap_or(&Self::INITIAL_SCORE);
        let new_score = current.saturating_sub(amount);
        scores.insert(user_id.to_string(), new_score);
        
        // イベント記録
        let mut events = self.events.lock().unwrap();
        events.push(TokuEvent {
            user_id: user_id.to_string(),
            delta: -(amount as i32),
            reason,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        
        log::info!("[Toku] {} の徳スコア: {} → {} (ペナルティ)", user_id, current, new_score);
    }

    /// ユーザーIDをハッシュ化（8バイト）
    pub fn hash_user_id(user_id: &str) -> [u8; 8] {
        // 簡易ハッシュ（本番では SHA-256 などを使用）
        let mut hash = [0u8; 8];
        let bytes = user_id.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            hash[i % 8] ^= b;
            hash[(i + 3) % 8] = hash[(i + 3) % 8].wrapping_add(b);
        }
        hash
    }

    /// 関係性を取得
    pub fn get_relationship(&self, from: &str, to: &str) -> Relationship {
        let rels = self.relationships.lock().unwrap();
        rels.get(&(from.to_string(), to.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// 関係性を更新（強さを加算）
    pub fn update_relationship(&self, from: &str, to: &str, delta: i32) {
        let mut rels = self.relationships.lock().unwrap();
        let key = (from.to_string(), to.to_string());
        
        let rel = rels.entry(key).or_default();
        
        // 強さを更新 (0 ~ 1000)
        let new_strength = (rel.strength as i32 + delta).max(0).min(1000) as u32;
        rel.strength = new_strength;
        
        // 最終接触時刻を更新
        rel.last_interaction = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        log::info!("[Relationship] {} -> {}: Strength = {}", from, to, rel.strength);
    }
}

impl Default for TokuManager {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバル徳スコアマネージャ
static TOKU_MANAGER: once_cell::sync::Lazy<TokuManager> = 
    once_cell::sync::Lazy::new(TokuManager::new);

/// AGNから呼び出される徳スコア加算関数
pub fn agn_add_toku(user_id: &str, amount: u32) {
    TOKU_MANAGER.add_toku(user_id, amount, TokuReason::HelpProvided);
}

/// AGNから呼び出される徳スコア減算関数
pub fn agn_subtract_toku(user_id: &str, amount: u32) {
    TOKU_MANAGER.subtract_toku(user_id, amount, TokuReason::Penalty);
}

/// AGNから呼び出される徳スコア取得関数
pub fn agn_get_toku(user_id: &str) -> u32 {
    TOKU_MANAGER.get_score(user_id)
}

/// AGNから呼び出される関係性取得関数
pub fn agn_get_bond(from: &str, to: &str) -> Relationship {
    TOKU_MANAGER.get_relationship(from, to)
}

/// AGNから呼び出される絆レベル取得関数
pub fn agn_get_bond_level(from: &str, to: &str) -> u32 {
    TOKU_MANAGER.get_relationship(from, to).level
}

/// AGNから呼び出される絆有無確認関数
pub fn agn_has_bond(from: &str, to: &str) -> bool {
    TOKU_MANAGER.get_relationship(from, to).has_bond()
}

/// AGNから呼び出される絆深化関数 (助け合い成功時)
pub fn agn_deepen_bond(from: &str, to: &str, amount: u32) {
    let mut rels = TOKU_MANAGER.relationships.lock().unwrap();
    let key = (from.to_string(), to.to_string());
    
    let rel = rels.entry(key).or_insert_with(Relationship::new_bond);
    rel.deepen(amount);
    
    log::info!("[Bond] {} ⇔ {}: Level {} (Strength {}, Helped {} times)", 
        from, to, rel.level, rel.strength, rel.help_count);
}

/// AGNから呼び出される関係性更新関数 (後方互換)
pub fn agn_update_bond(from: &str, to: &str, amount: i32) {
    TOKU_MANAGER.update_relationship(from, to, amount);
}

/// 検出されたピア情報
#[derive(Debug, Clone)]
pub struct DetectedPeer {
    /// ユーザーID（ハッシュ化）
    pub peer_id: String,
    /// ビーコンタイプ
    pub beacon_type: BeaconType,
    /// 推定距離（メートル）
    pub estimated_distance: f64,
    /// 信号強度（RSSI）
    pub rssi: i16,
    /// 最終検出時刻
    pub last_seen: Instant,
    /// 徳スコア
    pub toku_score: Option<u32>,
    /// カスタムペイロード
    pub payload: HashMap<String, String>,
}

/// ビーコン設定
#[derive(Debug, Clone)]
pub struct BeaconConfig {
    /// ビーコンタイプ
    pub beacon_type: BeaconType,
    /// 発信間隔（ミリ秒）
    pub interval_ms: u64,
    /// 発信時間（秒、Noneは無制限）
    pub duration_sec: Option<u64>,
    /// カスタムペイロード
    pub payload: HashMap<String, String>,
}

/// P2P通信レイヤーの状態
#[derive(Debug, Clone, PartialEq)]
pub enum P2PState {
    /// 初期化前
    Uninitialized,
    /// スキャン中
    Scanning,
    /// ビーコン発信中
    Broadcasting,
    /// スキャン＆発信中
    ScanningAndBroadcasting,
    /// 停止中
    Stopped,
}

/// P2P通信マネージャ
/// BLE/Wi-Fi Awareの抽象化レイヤー
pub struct P2PManager {
    /// 現在の状態
    state: Arc<Mutex<P2PState>>,
    /// 検出されたピアのキャッシュ
    detected_peers: Arc<Mutex<HashMap<String, DetectedPeer>>>,
    /// 現在のビーコン設定
    current_beacon: Arc<Mutex<Option<BeaconConfig>>>,
    /// ピアキャッシュのTTL（秒）
    peer_cache_ttl: Duration,
    /// フィードイベントのキャッシュ
    feed_cache: Arc<Mutex<Vec<SocialTokuEvent>>>,
}

impl P2PManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(P2PState::Uninitialized)),
            detected_peers: Arc::new(Mutex::new(HashMap::new())),
            current_beacon: Arc::new(Mutex::new(None)),
            peer_cache_ttl: Duration::from_secs(30),
            feed_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// BLE/Wi-Fi Awareの初期化
    /// 
    /// # プラットフォーム対応
    /// - macOS/iOS: CoreBluetooth
    /// - Android: Android BLE API
    /// - Linux: BlueZ
    /// - WASM: WebRTC フォールバック
    pub async fn initialize(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        
        // TODO: btleplugの初期化
        // #[cfg(not(target_arch = "wasm32"))]
        // {
        //     use btleplug::api::{Central, Manager as _};
        //     let manager = btleplug::platform::Manager::new().await
        //         .map_err(|e| format!("BLE初期化エラー: {}", e))?;
        // }
        
        log::info!("[P2P] 初期化完了");
        *state = P2PState::Stopped;
        Ok(())
    }

    /// ビーコンスキャン開始
    pub async fn start_scanning(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            P2PState::Uninitialized => {
                return Err("P2Pマネージャが初期化されていません".to_string());
            }
            P2PState::Broadcasting => {
                *state = P2PState::ScanningAndBroadcasting;
            }
            _ => {
                *state = P2PState::Scanning;
            }
        }
        
        log::info!("[P2P] スキャン開始");
        
        // TODO: btleplugでのスキャン実装
        // central.start_scan(ScanFilter::default()).await?;
        
        Ok(())
    }

    /// ビーコンスキャン停止
    pub fn stop_scanning(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            P2PState::Scanning => {
                *state = P2PState::Stopped;
            }
            P2PState::ScanningAndBroadcasting => {
                *state = P2PState::Broadcasting;
            }
            _ => {}
        }
        
        log::info!("[P2P] スキャン停止");
        Ok(())
    }

    /// ビーコン発信開始
    pub async fn start_broadcasting(&self, config: BeaconConfig) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            P2PState::Uninitialized => {
                return Err("P2Pマネージャが初期化されていません".to_string());
            }
            P2PState::Scanning => {
                *state = P2PState::ScanningAndBroadcasting;
            }
            _ => {
                *state = P2PState::Broadcasting;
            }
        }
        
        // ビーコン設定を保存
        {
            let mut beacon = self.current_beacon.lock().unwrap();
            *beacon = Some(config.clone());
        }
        
        log::info!("[P2P] ビーコン発信開始: {:?}", config.beacon_type);
        
        // TODO: BLE Peripheralモード実装
        // blusterやble-peripheral-rustを使用
        
        Ok(())
    }

    /// ビーコン発信停止
    pub fn stop_broadcasting(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            P2PState::Broadcasting => {
                *state = P2PState::Stopped;
            }
            P2PState::ScanningAndBroadcasting => {
                *state = P2PState::Scanning;
            }
            _ => {}
        }
        
        {
            let mut beacon = self.current_beacon.lock().unwrap();
            *beacon = None;
        }
        
        log::info!("[P2P] ビーコン発信停止");
        Ok(())
    }

    /// 近くのピアを取得（距離でフィルタ）
    pub fn get_nearby_peers(&self, max_distance: f64) -> Vec<DetectedPeer> {
        let peers = self.detected_peers.lock().unwrap();
        let now = Instant::now();
        
        peers.values()
            .filter(|p| {
                // TTLチェック
                now.duration_since(p.last_seen) < self.peer_cache_ttl &&
                // 距離フィルタ
                p.estimated_distance <= max_distance
            })
            .cloned()
            .collect()
    }

    /// 特定の状態のピアを取得
    pub fn get_peers_by_beacon_type(&self, beacon_type: &BeaconType, max_distance: f64) -> Vec<DetectedPeer> {
        self.get_nearby_peers(max_distance)
            .into_iter()
            .filter(|p| &p.beacon_type == beacon_type)
            .collect()
    }

    /// RSSIから距離を推定（簡易版）
    /// 
    /// 計算式: distance = 10 ^ ((TxPower - RSSI) / (10 * n))
    /// - TxPower: -59 (1mでのRSSI基準値)
    /// - n: 2.0 (環境係数、屋内は2-4)
    pub fn estimate_distance_from_rssi(rssi: i16, tx_power: i16) -> f64 {
        if rssi == 0 {
            return -1.0; // 不明
        }
        
        let n = 2.0; // 環境係数
        let ratio = (tx_power as f64 - rssi as f64) / (10.0 * n);
        10.0_f64.powf(ratio)
    }

    /// 現在の状態を取得
    pub fn get_state(&self) -> P2PState {
        self.state.lock().unwrap().clone()
    }

    /// ピア情報を手動で追加（テスト用）
    #[cfg(test)]
    pub fn add_mock_peer(&self, peer: DetectedPeer) {
        let mut peers = self.detected_peers.lock().unwrap();
        peers.insert(peer.peer_id.clone(), peer);
    }
}

// ============================================================
// 徳フィード (Social Toku Feed)
// ============================================================

use serde::{Deserialize, Serialize};

/// ソーシャル徳イベントの種類
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SocialEventType {
    /// 助け合い発生
    HelpGiven,
    /// 感謝（「ありがとう」）
    ThankYou,
    /// 徳の送付
    TokuSent { amount: u32 },
    /// すれ違い
    PassedBy,
}

/// ソーシャル徳イベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialTokuEvent {
    /// イベントID (UUID/Hash)
    pub id: String,
    /// 実行者ID
    pub actor_id: String,
    /// 対象者ID
    pub target_id: String,
    /// イベント種類
    pub event_type: SocialEventType,
    /// 場所 (Lat, Lon)
    pub location: Option<(f64, f64)>,
    /// タイムスタンプ
    pub timestamp: u64,
    /// メッセージ（オプション）
    pub message: Option<String>,
    /// 画像URL（オプション）
    pub image_url: Option<String>,
}

impl SocialTokuEvent {
    pub fn new(actor: &str, target: &str, event_type: SocialEventType) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // 簡易ID生成 (rand::randomを使用)
        let id = format!("{}-{}-{}-{}", actor, target, timestamp, rand::random::<u16>());
        
        Self {
            id,
            actor_id: actor.to_string(),
            target_id: target.to_string(),
            event_type,
            location: None, // 後で設定
            timestamp,
            message: None,
            image_url: None,
        }
    }
    
    pub fn with_location(mut self, lat: f64, lon: f64) -> Self {
        self.location = Some((lat, lon));
        self
    }
    
    pub fn with_message(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }

    pub fn with_image(mut self, url: &str) -> Self {
        self.image_url = Some(url.to_string());
        self
    }
}

// P2PManagerへの拡張（ゴシッププロトコル）
impl P2PManager {
    /// ソーシャルイベントをブロードキャスト（ゴシップ）
    pub fn broadcast_social_event(&self, event: SocialTokuEvent) {
        log::info!("[Gossip] イベント伝搬開始: {:?} from {}", event.event_type, event.actor_id);
        
        // TODO: ここで実際に周囲のピアにパケットを送信する処理
        // 現在はローカルのフィードに追加するのみ
        self.add_feed_event(event);
    }
    
    /// フィードにイベントを追加
    pub fn add_feed_event(&self, event: SocialTokuEvent) {
        // TokuManagerのイベントリストとは別に、UI表示用のフィードを管理する想定
        let mut cache = self.feed_cache.lock().unwrap();
        cache.insert(0, event); // 最新を先頭に
        if cache.len() > 100 {
            cache.pop();
        }
    }

    /// イベントIDからイベントを取得
    pub fn get_feed_event(&self, id: &str) -> Option<SocialTokuEvent> {
        let cache = self.feed_cache.lock().unwrap();
        cache.iter().find(|e| e.id == id).cloned()
    }

    /// 全フィードイベントを取得
    pub fn get_all_feed_events(&self) -> Vec<SocialTokuEvent> {
        let cache = self.feed_cache.lock().unwrap();
        cache.clone()
    }
    
    // シミュレーション: 周囲からイベントを受信
    pub fn simulate_incoming_gossip(&self) -> Vec<SocialTokuEvent> {
        // デモ用: ランダムにイベントを生成
        let mut events = Vec::new();
        
        // 5%の確率でイベント発生
        if rand::random::<f32>() < 0.05 {
            events.push(SocialTokuEvent::new(
                "unknown_hero", 
                "lost_tourist", 
                SocialEventType::HelpGiven
            ).with_message("道案内しました！"));
        }
        
        events
    }
}

impl Default for P2PManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// AGNインタプリタとの統合
// ============================================================

/// P2Pマネージャのグローバルインスタンス
static P2P_MANAGER: once_cell::sync::Lazy<P2PManager> = 
    once_cell::sync::Lazy::new(P2PManager::new);

/// AGNから呼び出される空間検索関数
pub async fn agn_spatial_search(max_distance: f64, filters: &[(String, String)]) -> Vec<DetectedPeer> {
    let mut results = P2P_MANAGER.get_nearby_peers(max_distance);
    
    // フィルタ適用
    for (key, value) in filters {
        if key == "状態" || key == "status" {
            let beacon_type = match value.as_str() {
                "暇" | "idle" => BeaconType::Idle,
                "助けて" | "help" => BeaconType::NeedHelp,
                "観光中" | "touring" => BeaconType::Touring,
                _ => BeaconType::Custom(0x00),
            };
            results.retain(|p| p.beacon_type == beacon_type);
        }
    }
    
    results
}

/// AGNから呼び出されるビーコン発信関数
pub async fn agn_broadcast_beacon(beacon_type_str: &str, duration_sec: Option<u64>) -> Result<(), String> {
    let beacon_type = match beacon_type_str {
        "暇" | "idle" => BeaconType::Idle,
        "助けて" | "help" => BeaconType::NeedHelp,
        "観光中" | "touring" => BeaconType::Touring,
        _ => BeaconType::Custom(0x00), // カスタムタイプのデフォルト
    };
    
    let config = BeaconConfig {
        beacon_type,
        interval_ms: 100, // 100ms間隔
        duration_sec,
        payload: HashMap::new(),
    };
    
    P2P_MANAGER.start_broadcasting(config).await
}

/// AGNから呼び出される通知関数
pub async fn agn_notify_peer(peer_id: &str, message: &str) -> Result<(), String> {
    // TODO: BLE GATT経由での通知実装
    log::info!("[P2P] 通知送信: peer={}, message={}", peer_id, message);
    Ok(())
}

/// AGNから呼び出されるフィードイベント取得関数
pub async fn agn_get_feed_event(id: &str) -> Option<SocialTokuEvent> {
    P2P_MANAGER.get_feed_event(id)
}

/// AGNから呼び出される全フィードイベント取得関数
pub async fn agn_get_all_feed_events() -> Vec<SocialTokuEvent> {
    P2P_MANAGER.get_all_feed_events()
}

/// テスト用：フィードイベントを注入
pub fn agn_inject_feed_event(event: SocialTokuEvent) {
    P2P_MANAGER.add_feed_event(event);
}

// ============================================================
// Proof of Kindness (PoK)
// ============================================================

/// 近接性の証明 (RSSIベース)
pub struct ProximityVerifier;

impl ProximityVerifier {
    /// RSSI履歴から「実際にそこにいた」確率(Confidence)を算出
    /// return: 0.0 (怪しい) ~ 1.0 (確実)
    pub fn verify_presence(rssi_log: &[i16]) -> f32 {
        if rssi_log.len() < 5 {
            return 0.2; // データ不足
        }

        // 1. 平均RSSI
        let sum: i32 = rssi_log.iter().map(|&x| x as i32).sum();
        let avg = sum as f32 / rssi_log.len() as f32;

        // 2. 分散 (Variance) - 自然な揺らぎがあるか？
        // 機械的な固定値（スプーフィング）は分散が0に近い
        let variance_sum: f32 = rssi_log.iter()
            .map(|&x| (x as f32 - avg).powi(2))
            .sum();
        let variance = variance_sum / rssi_log.len() as f32;

        // 判定ロジック
        let mut confidence: f32 = 0.5;

        // 距離判定: 近いほど信頼性が高い（遠くからのスプーフィングは難しい）
        if avg > -60.0 { confidence += 0.3; }      // 非常に近い (<1m)
        else if avg > -80.0 { confidence += 0.1; } // 近い (<5m)
        else if avg < -90.0 { confidence -= 0.1; } // 遠い/不安定

        // 揺らぎ判定: 
        // 分散が極端に小さい(0-1) => スプーフィングの疑い (-0.3)
        // 適度な揺らぎ(2-50) => 自然 (+0.2)
        // 大きすぎる揺らぎ(>100) => 安定していない (-0.1)
        if variance < 1.0 { confidence -= 0.4; }
        else if variance >= 2.0 && variance < 50.0 { confidence += 0.2; }
        
        confidence.clamp(0.0, 1.0)
    }
}

/// AGNから呼び出される近接証明検証関数
pub fn agn_verify_presence(rssi_history: Vec<i16>) -> f32 {
    ProximityVerifier::verify_presence(&rssi_history)
}


// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_manager_initialization() {
        let manager = P2PManager::new();
        assert_eq!(manager.get_state(), P2PState::Uninitialized);
    }

    #[test]
    fn test_rssi_distance_estimation() {
        // 1mでの理論値（TxPower = -59, RSSI = -59 → distance = 1m）
        let distance = P2PManager::estimate_distance_from_rssi(-59, -59);
        assert!((distance - 1.0).abs() < 0.01);
        
        // 10mでの理論値（RSSI ≈ -79）
        let distance_10m = P2PManager::estimate_distance_from_rssi(-79, -59);
        assert!((distance_10m - 10.0).abs() < 1.0);
    }

    #[test]
    fn test_nearby_peers_filter() {
        let manager = P2PManager::new();
        
        // モックピアを追加
        manager.add_mock_peer(DetectedPeer {
            peer_id: "peer1".to_string(),
            beacon_type: BeaconType::Idle,
            estimated_distance: 5.0,
            rssi: -65,
            last_seen: Instant::now(),
            toku_score: Some(100),
            payload: HashMap::new(),
        });
        
        manager.add_mock_peer(DetectedPeer {
            peer_id: "peer2".to_string(),
            beacon_type: BeaconType::NeedHelp,
            estimated_distance: 15.0,
            rssi: -80,
            last_seen: Instant::now(),
            toku_score: Some(50),
            payload: HashMap::new(),
        });
        
        // 10m以内のピアをフィルタ
        let nearby = manager.get_nearby_peers(10.0);
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].peer_id, "peer1");
        
        // 状態でフィルタ
        let idle_peers = manager.get_peers_by_beacon_type(&BeaconType::Idle, 100.0);
        assert_eq!(idle_peers.len(), 1);
    }

    // === ビーコンパケットテスト ===

    #[test]
    fn test_beacon_packet_serialize_deserialize() {
        let user_id = TokuManager::hash_user_id("test_user_123");
        let packet = EeyoBeaconPacket::new(BeaconType::Idle, 1000, user_id);
        
        let bytes = packet.to_bytes();
        assert_eq!(bytes[0], EeyoBeaconPacket::MAGIC); // マジックバイト
        assert_eq!(bytes[1], EeyoBeaconPacket::VERSION); // バージョン
        assert_eq!(bytes[2], BeaconType::Idle.to_byte()); // タイプ
        
        // デシリアライズ
        let decoded = EeyoBeaconPacket::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.beacon_type, BeaconType::Idle);
        assert_eq!(decoded.toku_score, 1000);
        assert_eq!(decoded.user_id, user_id);
    }

    #[test]
    fn test_beacon_packet_with_location() {
        let user_id = [0u8; 8];
        let packet = EeyoBeaconPacket::new(BeaconType::Touring, 500, user_id)
            .with_location(35.68, 139.69); // 東京
        
        let bytes = packet.to_bytes();
        assert_eq!(bytes[13] & 0x01, 0x01); // 位置情報フラグ
        
        let decoded = EeyoBeaconPacket::from_bytes(&bytes).unwrap();
        assert!(decoded.latitude.is_some());
        assert!(decoded.longitude.is_some());
        
        // 精度確認（約0.01度）
        let lat = decoded.latitude.unwrap() as f64 / 100.0;
        let lon = decoded.longitude.unwrap() as f64 / 100.0;
        assert!((lat - 35.68).abs() < 0.01);
        assert!((lon - 139.69).abs() < 0.01);
    }

    // === 徳スコアテスト ===

    #[test]
    fn test_toku_manager_initial_score() {
        let manager = TokuManager::new();
        assert_eq!(manager.get_score("new_user"), TokuManager::INITIAL_SCORE);
    }

    #[test]
    fn test_toku_manager_add_and_get() {
        let manager = TokuManager::new();
        manager.add_toku("user1", 50, TokuReason::HelpProvided);
        
        // 初期値(100) + 50 = 150
        assert_eq!(manager.get_score("user1"), 150);
    }

    #[test]
    fn test_toku_manager_max_score() {
        let manager = TokuManager::new();
        manager.add_toku("user1", 100000, TokuReason::HelpProvided);
        
        // 最大値を超えない
        assert_eq!(manager.get_score("user1"), TokuManager::MAX_SCORE);
    }

    #[test]
    fn test_toku_manager_subtract() {
        let manager = TokuManager::new();
        manager.subtract_toku("user1", 50, TokuReason::Penalty);
        
        // 初期値(100) - 50 = 50
        assert_eq!(manager.get_score("user1"), 50);
    }

    #[test]
    fn test_user_id_hash() {
        let hash1 = TokuManager::hash_user_id("user_abc");
        let hash2 = TokuManager::hash_user_id("user_abc");
        let hash3 = TokuManager::hash_user_id("user_xyz");
        
        assert_eq!(hash1, hash2); // 同じ入力は同じ出力
        assert_ne!(hash1, hash3); // 異なる入力は異なる出力
    }

    #[test]
    fn test_secure_packet_roundtrip() {
        let context = SecurityContext::new();
        let public_key = context.verifying_key.to_bytes();
        let signing_key = &context.signing_key;
        
        // Create Packet
        let packet = EeyoSecurePacket::new(
            BeaconType::Idle,
            1234,
            &public_key,
            signing_key
        );
        
        // Serialize
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), EeyoSecurePacket::PACKET_SIZE, "Packet size mismatch");
        
        // Deserialize & Verify
        let parsed = EeyoSecurePacket::from_bytes(&bytes);
        assert!(parsed.is_some(), "Failed to parse/verify valid packet");
        
        let p = parsed.unwrap();
        assert_eq!(p.toku_score, 1234);
        assert_eq!(p.beacon_type, BeaconType::Idle);
        assert_eq!(p.sender_public_key, public_key);
    }

    #[test]
    fn test_tampered_packet() {
        let context = SecurityContext::new();
        let pub_key = context.verifying_key.to_bytes();
        let packet = EeyoSecurePacket::new(BeaconType::NeedHelp, 500, &pub_key, &context.signing_key);
        let mut bytes = packet.to_bytes();
        
        // Tamper with Toku Score (byte 4)
        bytes[4] = bytes[4].wrapping_add(1);
        
        let parsed = EeyoSecurePacket::from_bytes(&bytes);
        assert!(parsed.is_none(), "Tampered packet should fail verification");
    }
}
