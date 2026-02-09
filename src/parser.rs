//! AGN Parser - 構文解析器
//! 日本語SOV構文と英語SVO構文の両方を解析してASTを生成する

use crate::lexer::Token;

/// 式（値を表す）
#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Variable(String),
    // Eeyo: 空間・時間型 (Phase 13)
    Distance { value: f64, unit: String },
    Duration { value: f64, unit: String },
    // AGN 2.0: Property Access (User.Toku)
    PropertyAccess {
        target: Box<Expr>,
        property: String,
    },
    // AGN 2.0: Bond (Bond between two entities)
    Bond(Box<Expr>, Box<Expr>),
    // AGN 2.0: Call (Action/Rule call as expression)
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

/// 条件式
#[derive(Debug, Clone)]
pub enum Condition {
    Equals(Expr, Expr),
    GreaterThan(Expr, Expr),
    LessThan(Expr, Expr),
    // Eeyo: 空間条件
    Nearer(Expr),   // より近い
    Farther(Expr),  // より遠い
    // AGN 2.0: 関係性条件
    HasBond(Expr, Expr), // A と B の間に 絆 がある
    // Truthy check
    Truthy(Expr),
}

/// 空間検索フィルター
#[derive(Debug, Clone)]
pub struct SpatialFilter {
    pub field: String,       // "状態", "徳"
    pub condition: Condition,
}

/// 文（実行単位）
#[derive(Debug, Clone)]
pub enum Statement {
    /// 代入文: [ターゲット] は [値] だ / let X = 10
    Assignment { target: Expr, value: Expr },
    /// アセットロード: [ターゲット] は [パス] を 読み込む
    LoadAsset {
        target: Expr,
        path: Expr,
    },
    /// UIコンポーネント定義: [ターゲット] は [スタイル] な [コンポーネント] だ
    ComponentDefine {
        target: Expr,
        style: String,
        component: String,
    },
    /// 二項演算: [ターゲット] に [値] を [動詞] / add [値] to [ターゲット]
    BinaryOp { target: Expr, operand: Expr, verb: String },
    /// 単項関数: [値] を [動詞] / show [値]
    UnaryOp { operand: Expr, verb: String },
    /// 非同期実行: [値] を 並列で [動詞]
    AsyncOp { operand: Expr, verb: String },
    /// 条件分岐: if [条件] then [処理] end
    IfStatement {
        condition: Condition,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    /// ループ: repeat [回数] times [処理] end / [回数] 回 繰り返す [処理] おわり
    RepeatStatement {
        count: Expr,
        body: Vec<Statement>,
    },
    /// AI操作: [ターゲット] は [入力] を [オプション] に [動詞]
    AiOp {
        result: Expr,
        input: Expr,
        verb: String,
        options: Option<Expr>,
    },
    /// 画面出力: [値] を 画面 に 表示する / show [値] to Screen
    ScreenOp {
        operand: Expr,
    },
    /// イベントハンドラ: on [ターゲット] click ... end / [ターゲット] を 押したとき ... おわり
    EventHandler {
        target: Expr,
        event: String,
        body: Vec<Statement>,
    },
    /// Phase 15: Event Listener: on Event(Type) from A to B
    EventListener {
        event_type: String,
        from_var: Option<String>,
        to_var: Option<String>,
        body: Vec<Statement>,
    },
    /// 遅延実行: [時間] 後 に ... おわり / after [Time] ... end
    DelayStatement {
        duration: Expr,
        body: Vec<Statement>,
    },
    /// アニメーション: [時間] かけて [ターゲット] の [プロパティ] を [値] に する
    AnimateStatement {
        duration: Expr,
        target: Expr,
        property: String, // "色", "サイズ", "影"
        value: Expr,
    },
    
    // === Phase 10: Vector Graphics & UI ===
    /// ブロック: [ターゲット] の 中 に ... おわり
    Block {
        target: Expr,
        body: Vec<Statement>,
    },
    /// レイアウト: [リスト] を [方向] に 置く
    Layout {
        target: Expr,
        direction: LayoutDirection,
    },
    
    // === Eeyo: 空間・通信 (Phase 13) ===
    /// 空間検索: [結果] は [距離] より 近い 人 で [条件] な 人 を 探す
    SpatialSearch {
        result: Expr,
        max_distance: Expr,
        filters: Vec<SpatialFilter>,
    },
    /// ビーコン発信: ビーコン を 発信する ... おわり
    BeaconBroadcast {
        beacon_type: String,
        duration: Option<Expr>,
        payload: Vec<(String, Expr)>,
    },
    /// 助けを求める / 通知: [ターゲット] に [メッセージ] を 通知する
    Notify {
        target: Expr,
        message: Expr,
    },
    /// 徳の付与: [ターゲット] に [量] だけ 徳 を 加算する
    TokuAccrue {
        target: Expr,
        amount: Expr,
    },

    // === AGN 2.0 (Social Layer) ===
    /// ルール定義: ルール [名前] ... おわり
    RuleDefinition {
        name: String,
        body: Vec<Statement>,
    },
    /// アクション定義: アクション [名前] ... おわり
    ActionDefinition {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    /// 変数更新 (再代入): [式] を [値] に 更新する / 増やす / 減らす
    VariableUpdate {
        target: Expr, // Can be PropertyAccess
        value: Expr,
        verb: String, // "更新する", "増やす", "減らす"
    },
    /// リターン: 結果 を [式] とする
    ReturnStatement {
        value: Expr,
    },
    /// アクション呼び出し: 徳を送る(送信者, 受信者, 10)
    ActionCall {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

/// プログラム全体
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn peek(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current(), Token::Newline) {
            self.advance();
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;
        
        // Postfix operators (Property Access)
        while matches!(self.current(), Token::Dot) {
            self.advance(); // skip dot
            
            let property = match self.current() {
                Token::Noun(n) => n.clone(),
                Token::KeywordToku => "徳".to_string(),
                Token::KeywordRank => "ランク".to_string(),
                Token::KeywordBond => "絆".to_string(),
                _ => return Err(format!("Expected property name after dot, got {:?}", self.current())),
            };
            self.advance();
            
            left = Expr::PropertyAccess {
                target: Box::new(left),
                property,
            };
        }
        
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.current().clone();
        
        let expr = match token {
            Token::Number(n) => {
                self.advance();
                Expr::Number(n)
            },
            Token::String(s) => {
                self.advance();
                Expr::String(s)
            },
            Token::Noun(name) => {
                self.advance();
                if matches!(self.current(), Token::LParen) {
                    self.advance(); // skip (
                    let mut args = Vec::new();
                    loop {
                        if matches!(self.current(), Token::RParen) {
                            self.advance();
                            break;
                        }
                        args.push(self.parse_expression()?);
                        
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        } else if !matches!(self.current(), Token::RParen) {
                            return Err(format!("Expected ',' or ')' in call, got {:?}", self.current()));
                        }
                    }
                    Expr::Call { name: name.clone(), args }
                } else {
                    Expr::Variable(name.clone())
                }
            },
            Token::Distance { value, unit } => {
                self.advance();
                Expr::Distance { value, unit }
            },
            Token::Duration { value, unit } => {
                self.advance();
                Expr::Duration { value, unit }
            },
            Token::KeywordBond => {
                // bond(Expr, Expr)
                self.advance(); // skip bond
                if !matches!(self.current(), Token::LParen) {
                    return Err(format!("Expected '(' after bond, got {:?}", self.current()));
                }
                self.advance(); // skip (
                
                let left = self.parse_expression()?;
                
                if !matches!(self.current(), Token::Comma) {
                    return Err(format!("Expected ',' in bond(), got {:?}", self.current()));
                }
                self.advance(); // skip ,
                
                let right = self.parse_expression()?;
                
                if !matches!(self.current(), Token::RParen) {
                    return Err(format!("Expected ')' after bond(), got {:?}", self.current()));
                }
                self.advance(); // skip )
                
                Expr::Bond(Box::new(left), Box::new(right))
            }
            _ => return Err(format!("Expected expression, got {:?}", token)),
        };
        
        Ok(expr)
    }

    fn current_to_expr(&mut self) -> Result<Expr, String> {
        self.parse_expression()
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        loop {
            self.skip_newlines();

            if matches!(self.current(), Token::EOF) {
                break;
            }

            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.skip_newlines();

        // AGN 2.0: Rule Definition
        if matches!(self.current(), Token::KeywordRule) {
            return self.parse_rule_definition();
        }
        
        // AGN 2.0: Action Definition
        if matches!(self.current(), Token::KeywordAction) {
            return self.parse_action_definition();
        }
        
        // AGN 2.0: Event Listener
        if matches!(self.current(), Token::KeywordOn) {
            // Check if next is Event keyword
            if matches!(self.peek(1), Token::KeywordEvent) {
                return self.parse_event_listener();
            } else {
                // Regular UI event handler (on Button click)
                return self.parse_event_handler();
            }
        }
        
        // AGN 2.0: English Action Commands (increase, decrease, update)
        if matches!(self.current(), Token::KeywordIncrease | Token::KeywordDecrease | Token::KeywordUpdate) {
             return self.parse_english_action_command();
        }
        
        // === English SVO Patterns ===
        
        // English: show X to Screen (check before regular show)
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") 
            && self.look_for_screen_target() {
            return self.parse_show_to_screen();
        }
        
        // English: show X / print X
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") {
            return self.parse_english_show();
        }
        
        // English: summarize X / translate X (AI verbs)
        if matches!(self.current(), Token::Verb(v) if v == "summarize" || v == "translate") {
            return self.parse_english_ai_verb();
        }
        
        // English: add X to Y
        if matches!(self.current(), Token::Verb(v) if v == "add" || v == "subtract" || v == "multiply" || v == "divide") {
            return self.parse_english_binary_op();
        }
        
        // English: if X equals Y then ... end
        if matches!(self.current(), Token::KeywordIf) {
            return self.parse_if_statement();
        }
        
        // English: repeat N times ... end
        if matches!(self.current(), Token::KeywordRepeat) {
            return self.parse_english_repeat();
        }
        
        // English: let X = 10 (optional)
        if matches!(self.current(), Token::KeywordLet) {
            return self.parse_english_let();
        }
        
        // English: X is 10 (like Japanese X は 10 だ)
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::KeywordIs) {
            return self.parse_english_is_assignment();
        }
        
        // === Japanese SOV Patterns ===
        
        // 日本語: [名詞] は [値] だ / [パス] を 読み込む / [スタイル] な [コン] だ
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::ParticleWa) {
            return self.parse_assignment();
        }
        
        // AGN 2.0: Property Access or Function Lead Start (User.Toku ... / bond(...) ...)
        if matches!(self.current(), Token::Noun(_) | Token::KeywordBond | Token::KeywordRank) 
           && (matches!(self.peek(1), Token::Dot | Token::LParen)) {
             return self.parse_expr_statement();
        }

        // 日本語: [名詞] に [値] を [動詞]
        if matches!(self.current(), Token::Noun(_)) && matches!(self.peek(1), Token::ParticleNi) {
            return self.parse_binary_op();
        }

        // Phase 10: [名詞] を [方向] に 置く
        // Check if Noun leads to Layout pattern: Noun + Wo + Direction
        // Must check BEFORE UnaryOrAsync because both start with Noun + Wo
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && self.is_direction_token(self.peek(2)) {
             return self.parse_layout();
        }

        // Phase 12: [名詞] を 押したとき / 動かしたとき
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && (matches!(self.peek(2), Token::KeywordClick | Token::KeywordDrag)) {
             return self.parse_object_event();
        }
        
        if matches!(self.peek(1), Token::ParticleWo) {
             // println!("Debug: Noun+Wo detected. peek2={:?}", self.peek(2));
        }

        // 日本語: [対象] を [ターゲット] に [動詞] (逆順: O を T に V)
        if matches!(self.current(), Token::Noun(_)) 
           && matches!(self.peek(1), Token::ParticleWo)
           && (matches!(self.peek(2), Token::Noun(_) | Token::ScreenNoun) && matches!(self.peek(3), Token::ParticleNi)) {
             return self.parse_binary_op_reverse();
        }

        // 日本語: [値] を (並列で)? [動詞]
        if matches!(self.peek(1), Token::ParticleWo) {
            // Unary, Async, or Binary Op
            return self.parse_unary_or_async_op();
        }
        
        // 日本語: [数値] 回 繰り返す ... おわり
        if matches!(self.current(), Token::Number(_)) && matches!(self.peek(1), Token::KeywordTimes) {
            return self.parse_japanese_repeat();
        }
        
        // 日本語: [時間] 秒 後 に / [時間] 秒 かけて
        if matches!(self.current(), Token::Number(_)) && matches!(self.peek(1), Token::KeywordSeconds) {
            return self.parse_timed_statement();
        }
        
        // 日本語: もし ... ならば ... おわり
        if matches!(self.current(), Token::KeywordIf) {
            return self.parse_if_statement();
        }
        
        // === Phase 6: UI Patterns ===
        
        if matches!(self.current(), Token::KeywordOn) {
            return self.parse_on_event();
        }
        
        // Phase 10: [名詞] の 中 に ... おわり
        // Check for Noun followed by ParticleNo + KeywordInside + ParticleNi?
        // Actually Lexer returns KeywordInside for "の中".
        // Pattern: [Noun] [KeywordInside] [ParticleNi]
        // Phase 10: [名詞] の 中 に ... おわり
        // Phase 10: [名詞] の 中 に ... おわり
        // Support both "の中" (KeywordInside) and "の" "中" (ParticleNo + Noun("中"))
        if matches!(self.current(), Token::Noun(_)) {
            if matches!(self.peek(1), Token::KeywordInside) {
                return self.parse_block();
            }
            if matches!(self.peek(1), Token::ParticleNo) 
               && matches!(self.peek(2), Token::Noun(n) if n == "中") {
                return self.parse_block();
            }
        }

        // Phase 11: [数値] 秒 かけて [プロパティ] を [値] にする / 深くする
        if matches!(self.current(), Token::Number(_)) && matches!(self.peek(1), Token::KeywordSeconds) {
             return self.parse_animate();
        }
        
        // Phase 11: マウス が 上 に あるとき
        if matches!(self.current(), Token::KeywordMouse) {
             return self.parse_mouse_event();
        }
        

        
        // English: show X to Screen
        if matches!(self.current(), Token::Verb(v) if v == "show" || v == "print") 
            && self.look_for_screen_target() {
            return self.parse_show_to_screen();
        }

        Err(format!("Unexpected token: {:?}", self.current()))
    }

    // === English SVO Parsers ===
    
    fn parse_english_show(&mut self) -> Result<Statement, String> {
        // show X / print X
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance(); // skip verb
        
        let operand = self.current_to_expr()?;
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "show" | "print" => "表示する".to_string(),
            _ => verb,
        };
        
        Ok(Statement::UnaryOp { operand, verb: normalized_verb })
    }
    
    fn parse_english_ai_verb(&mut self) -> Result<Statement, String> {
        // summarize X / translate X
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected AI verb".to_string()),
        };
        self.advance(); // skip verb
        
        let input = self.current_to_expr()?;
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "summarize" => "要約する".to_string(),
            "translate" => "翻訳する".to_string(),
            _ => verb,
        };
        
        // This returns a value, so we need an assignment context
        // For now, treat as UnaryOp that returns a value
        Ok(Statement::UnaryOp { operand: input, verb: normalized_verb })
    }
    
    fn parse_english_binary_op(&mut self) -> Result<Statement, String> {
        // add X to Y / subtract X from Y
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance(); // skip verb
        
        let operand = self.current_to_expr()?;
        
        // Expect "to"
        if !matches!(self.current(), Token::KeywordTo) {
            return Err("Expected 'to'".to_string());
        }
        self.advance(); // skip to
        
        let target = self.parse_expression()?;
        
        // Normalize to Japanese verb
        let normalized_verb = match verb.as_str() {
            "add" => "足す".to_string(),
            "subtract" => "引く".to_string(),
            "multiply" => "掛ける".to_string(),
            "divide" => "割る".to_string(),
            _ => verb,
        };
        
        Ok(Statement::BinaryOp { target, operand, verb: normalized_verb })
    }
    
    fn parse_english_let(&mut self) -> Result<Statement, String> {
        // let X = 10
        self.advance(); // skip let
        
        let _name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected variable name".to_string()),
        };
        self.advance(); // skip name
        
        // Expect "is" or "="
        if matches!(self.current(), Token::KeywordIs) {
            self.advance();
        }
        
        let target = self.parse_expression()?;
        
        if !matches!(self.current(), Token::KeywordEquals) {
            return Err("Expected '='".to_string());
        }
        self.advance(); // skip =
        
        let value = self.current_to_expr()?;
        
        Ok(Statement::Assignment { target, value })
    }
    
    fn parse_english_is_assignment(&mut self) -> Result<Statement, String> {
        // X is 10 (like Japanese X は 10 だ)
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected variable name".to_string()),
        };
        self.advance(); // skip name
        
        // Expect "is"
        if !matches!(self.current(), Token::KeywordIs) {
            return Err("Expected 'is'".to_string());
        }
        self.advance(); // skip is
        
        let value = self.current_to_expr()?;
        
        Ok(Statement::Assignment { target: Expr::Variable(name), value })
    }
    
    fn parse_english_repeat(&mut self) -> Result<Statement, String> {
        // repeat N times ... end
        self.advance(); // skip repeat
        
        let count = self.current_to_expr()?;
        
        // Expect "times"
        if !matches!(self.current(), Token::KeywordTimes) {
            return Err("Expected 'times'".to_string());
        }
        self.advance(); // skip times
        
        // Parse body until "end"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::RepeatStatement { count, body })
    }
    
    fn parse_if_statement(&mut self) -> Result<Statement, String> {
        // if X equals Y then ... end
        // もし X と等しい Y ならば ... おわり
        // もし A と B の間に 絆 がある ならば ...
        self.advance(); // skip if / もし
        
        let condition = if matches!(self.peek(1), Token::ParticleTo) {
            // 日本語絆構文: [Expr] と [Expr] ... 絆 がある
            let left = self.parse_expression()?;
            self.advance(); // skip と
            let right = self.parse_expression()?;
            
            // Skip "の間に" etc.
            while matches!(self.current(), Token::ParticleNo | Token::KeywordInside | Token::KeywordBetween | Token::ParticleNi | Token::KeywordPerson) {
                self.advance();
            }
            
            if !matches!(self.current(), Token::KeywordBond) {
                return Err(format!("Expected '絆' in relationship condition, got {:?}", self.current()));
            }
            self.advance(); // skip 絆
            
            if matches!(self.current(), Token::KeywordAre | Token::KeywordIs | Token::ParticleDa | Token::ParticleGa) {
                self.advance(); // consume がある / だ
                if matches!(self.current(), Token::KeywordAre) {
                    self.advance(); // consume "ある" if "が" "ある"
                }
            }
            
            Condition::HasBond(left, right)
        } else {
            // Standard Condition: left OP right
            let left = self.current_to_expr()?;
            
            match self.current() {
                Token::KeywordEquals => {
                    self.advance();
                    let right = self.current_to_expr()?;
                    Condition::Equals(left, right)
                }
                Token::KeywordGreaterThan => {
                    self.advance();
                    let right = self.current_to_expr()?;
                    Condition::GreaterThan(left, right)
                }
                Token::KeywordLessThan => {
                    self.advance();
                    let right = self.current_to_expr()?;
                    Condition::LessThan(left, right)
                }
                Token::KeywordThen | Token::KeywordEnd | Token::Newline | Token::EOF => {
                    // No operator: Truthy check (e.g. `if bond(A, B) then`)
                    Condition::Truthy(left)
                }
                _ => return Err(format!("Expected comparison operator, got {:?}", self.current())),
            }
        };
        
        // Expect "then" / "ならば"
        if !matches!(self.current(), Token::KeywordThen) {
            return Err("Expected 'then' or 'ならば'".to_string());
        }
        self.advance(); // skip then
        
        // Parse then block
        let then_block = self.parse_block_until_end_or_else()?;
        
        // Check for else
        let else_block = if matches!(self.current(), Token::KeywordElse) {
            self.advance(); // skip else
            Some(self.parse_block_until_end()?)
        } else {
            None
        };
        
        // "end" is consumed by parse_block_until_end
        
        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
        })
    }
    
    fn parse_japanese_repeat(&mut self) -> Result<Statement, String> {
        // N 回 繰り返す ... おわり
        let count = self.current_to_expr()?;
        
        // Expect "回"
        if !matches!(self.current(), Token::KeywordTimes) {
            return Err("Expected '回'".to_string());
        }
        self.advance(); // skip 回
        
        // Expect "繰り返す"
        if !matches!(self.current(), Token::Verb(v) if v == "繰り返す") {
            return Err("Expected '繰り返す'".to_string());
        }
        self.advance(); // skip 繰り返す
        
        // Parse body until "おわり"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::RepeatStatement { count, body })
    }
    
    // === Phase 6: UI Parsing ===
    
    fn is_direction_token(&self, token: &Token) -> bool {
        matches!(token, Token::KeywordVertical | Token::KeywordHorizontal) ||
        matches!(token, Token::Noun(n) if n == "縦並び" || n == "横並び")
    }
    
    /// Screen出力を先読みでチェック
    fn look_for_screen_target(&self) -> bool {
        // show X to Screen パターンを検出
        for i in 0..5 {
            if matches!(self.peek(i), Token::ScreenNoun) {
                return true;
            }
        }
        false
    }
    
    fn parse_on_event(&mut self) -> Result<Statement, String> {
        // on Button click ... end
        self.advance(); // skip on
        
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected element name".to_string()),
        };
        self.advance(); // skip target
        
        // Expect "click"
        let event = if matches!(self.current(), Token::KeywordClick) {
            self.advance();
            "click".to_string()
        } else {
            return Err("Expected 'click'".to_string());
        };
        
        // Parse body until "end"
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::EventHandler { target: Expr::Variable(target), event, body })
    }
    
    fn parse_show_to_screen(&mut self) -> Result<Statement, String> {
        // show X to Screen
        self.advance(); // skip show/print
        
        let operand = self.current_to_expr()?;
        
        // Expect "to"
        if !matches!(self.current(), Token::KeywordTo) {
            return Err("Expected 'to'".to_string());
        }
        self.advance(); // skip to
        
        // Expect "Screen"
        if !matches!(self.current(), Token::ScreenNoun) {
            return Err("Expected 'Screen'".to_string());
        }
        self.advance(); // skip Screen
        
        Ok(Statement::ScreenOp { operand })
    }
    
    fn parse_block_until_end(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        loop {
            self.skip_newlines();
            
            if matches!(self.current(), Token::KeywordEnd | Token::EOF) {
                if matches!(self.current(), Token::KeywordEnd) {
                    self.advance(); // skip end
                }
                break;
            }
            
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        
        Ok(statements)
    }
    
    fn parse_block_until_end_or_else(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        loop {
            self.skip_newlines();
            
            if matches!(self.current(), Token::KeywordEnd | Token::KeywordElse | Token::EOF) {
                if matches!(self.current(), Token::KeywordEnd) {
                    self.advance(); // skip end
                }
                break;
            }
            
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        
        Ok(statements)
    }

    // === Phase 10 Parsers ===

    fn parse_block(&mut self) -> Result<Statement, String> {
        // [名詞] の 中 に ... おわり
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance(); // skip noun
        
        // Expect "の中" (KeywordInside) OR "の" "中"
        if matches!(self.current(), Token::KeywordInside) {
            self.advance();
        } else if matches!(self.current(), Token::ParticleNo) {
            self.advance();
            // Expect "中"
            match self.current() {
                Token::Noun(n) if n == "中" => {
                    self.advance();
                }
                _ => return Err("Expected '中' after 'の'".to_string()),
            }
        } else {
            return Err("Expected 'の中' or 'の 中'".to_string());
        }
        
        // Expect "に" (ParticleNi)
        if !matches!(self.current(), Token::ParticleNi) {
             return Err("Expected 'に'".to_string());
        }
        self.advance();
        
        // Parse body
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::Block { target: Expr::Variable(target), body })
    }

    fn parse_layout(&mut self) -> Result<Statement, String> {
        // [名詞] を [方向] に 置く
        let target = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected noun".to_string()),
        };
        self.advance(); // skip target
        
        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();
        
        let direction = match self.current() {
            Token::KeywordVertical => LayoutDirection::Vertical,
            Token::KeywordHorizontal => LayoutDirection::Horizontal,
            Token::Noun(n) if n == "縦並び" => LayoutDirection::Vertical, // Fallback if lexer didn't catch
            Token::Noun(n) if n == "横並び" => LayoutDirection::Horizontal,
            _ => return Err("Expected '縦並び' or '横並び'".to_string()),
        };
        self.advance();
        
        // Expect "に" (ParticleNi)
        if matches!(self.current(), Token::ParticleNi) {
             self.advance();
        }
        
        // For robustness, handle both.
        match self.current() {
            Token::Verb(v) if v == "置く" => self.advance(),
            Token::Noun(n) if n == "置く" => self.advance(),
            _ => return Err("Expected '置く'".to_string()),
        };
        
        Ok(Statement::Layout { target: Expr::Variable(target), direction })
    }

    // === Phase 11: Animation & Event Parsers ===

    fn parse_animate(&mut self) -> Result<Statement, String> {
        // [数値] 秒 かけて [プロパティ] を [値] にする / 深くする
        let duration = match self.current() {
            Token::Number(n) => *n,
            _ => return Err("Expected duration number".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::KeywordSeconds) {
            return Err("Expected '秒'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::KeywordOver) {
            return Err("Expected 'かけて'".to_string());
        }
        self.advance();

        // Property: "影" or "背景" (Noun)
        let property = match self.current() {
             Token::KeywordShadow => "shadow".to_string(), // "影"
             Token::Noun(n) => n.clone(),
             _ => return Err("Expected property name (影 or noun)".to_string()),
        };
        self.advance();

        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();

        // Target Value: "深くする" (Deepen) or "[Value] にする"
        let target_value = if matches!(self.current(), Token::KeywordDeepen) {
             self.advance();
             Expr::String("deepen".to_string()) // Special value
        } else {
             // [Value] にする
             let val = self.current_to_expr()?;
             
             if matches!(self.current(), Token::ParticleNi) {
                 self.advance();
             }
             
             if !matches!(self.current(), Token::KeywordChange) {
                 // Check verb "する"?
                 match self.current() {
                     Token::Verb(v) if v == "する" => { self.advance(); },
                     Token::Noun(n) if n == "する" => { self.advance(); },
                     Token::KeywordChange => { self.advance(); },
                     Token::Noun(n) if n == "にする" => { self.advance(); },
                     _ => return Err("Expected 'にする' or 'する'".to_string()),
                 }
             } else {
                 self.advance(); // skip 'change'
             }
             val
        };

        Ok(Statement::AnimateStatement { 
            duration: Expr::Number(duration), 
            target: Expr::Variable("Unknown".to_string()),
            property, 
            value: target_value 
        })
    }

    fn parse_mouse_event(&mut self) -> Result<Statement, String> {
        // マウス が 上 に あるとき
        // Token::KeywordMouse has been consumed? No, parser dispatcher peeks.
        self.advance(); // skip Mouse

        // Skip optional particle "ga"
        if matches!(self.current(), Token::Noun(n) if n == "が") {
            self.advance();
        }

        if !matches!(self.current(), Token::KeywordAbove) { // "上"
             return Err("Expected '上'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::ParticleNi) { // "に"
             return Err("Expected 'に'".to_string());
        }
        self.advance();

        if !matches!(self.current(), Token::KeywordWhen) { // "あるとき"
             return Err("Expected 'あるとき'".to_string());
        }
        self.advance();

        let body = self.parse_block_until_end()?;

        Ok(Statement::EventHandler { 
            target: Expr::Variable("self".to_string()), // Implicit target (current component)
            event: "hover".to_string(), 
            body 
        })
    }

    fn parse_object_event(&mut self) -> Result<Statement, String> {
        // [式] を 押したとき / 動かしたとき
        let target = self.parse_expression()?;
        
        // Expect "を"
        if !matches!(self.current(), Token::ParticleWo) {
             return Err("Expected 'を'".to_string());
        }
        self.advance();
        
        let event = match self.current() {
            Token::KeywordClick => "click".to_string(),
            Token::KeywordDrag => "drag".to_string(),
            _ => return Err("Expected event type (押したとき/動かしたとき)".to_string()),
        };
        self.advance();
        
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::EventHandler { target, event, body })
    }

    // === Japanese SOV Parsers (existing) ===
    
    fn parse_assignment(&mut self) -> Result<Statement, String> {
        // [式] は [値] だ
        let target = self.parse_expression()?;
        
        if !matches!(self.current(), Token::ParticleWa) {
            return Err("Expected 'は'".to_string());
        }
        self.advance(); // skip は
      let expr = self.current_to_expr()?;

        match self.current() {
            Token::ParticleDa => {
                // [値] だ
                self.advance();
                Ok(Statement::Assignment { target, value: expr })
            }
            Token::ParticleWo => {
                // [パス] を 読み込む OR [値] を 翻訳する/要約する
                self.advance();
                match self.current() {
                    Token::Verb(v) if v == "読み込む" => {
                        self.advance();
                        Ok(Statement::LoadAsset { target: target.clone(), path: expr })
                    }
                    Token::Verb(v) if v == "翻訳する" || v == "要約する" || v == "想像する" => {
                        // AI verb in assignment: 結果 は テキスト を 翻訳する
                        let verb = v.clone();
                        self.advance();
                        Ok(Statement::AiOp { 
                            result: target.clone(), 
                            input: expr, 
                            verb,
                            options: None,
                        })
                    }
                    _ => {
                        // Try optional argument: [Input] を [Option] に [Verb]
                        let options_expr = self.current_to_expr()?;
                        
                        if !matches!(self.current(), Token::ParticleNi) {
                            return Err("Expected '読み込む', AI verb, or option followed by 'に'".to_string());
                        }
                        self.advance(); // skip に
                        
                        match self.current() {
                             Token::Verb(v) if v == "翻訳する" || v == "要約する" => {
                                let verb = v.clone();
                                self.advance();
                                Ok(Statement::AiOp { 
                                    result: target.clone(), 
                                    input: expr, 
                                    verb, 
                                    options: Some(options_expr) 
                                })
                             }
                             _ => Err("Expected AI Verb after option".to_string())
                        }
                    }
                }
            }
            Token::ParticleNa | Token::ParticleNo => {
                // [スタイル] な/の [コンポーネント] だ
                self.advance(); // skip Na/No
                // Expect Component Noun
                let component = match self.current() {
                     Token::Noun(n) => n.clone(),
                     _ => return Err("Expected component noun".to_string()),
                };
                self.advance();
                
                if !matches!(self.current(), Token::ParticleDa) {
                     return Err("Expected 'だ'".to_string());
                }
                self.advance();
                
                // Extract style from expr
                let style = match expr {
                    Expr::Variable(s) => s,
                    Expr::String(s) => s, // Allow string style (e.g. content text?)
                                          // Actually style usually is "Red". Content is different.
                                          // But for now, treating Expr as style.
                    _ => return Err("Expected style variable or string".to_string()),
                };
                    Ok(Statement::ComponentDefine { target: target.clone(), style, component })
            }
            Token::Noun(component) => {
                 // [スタイル] [コンポーネント] だ (implicit "な")
                 let component = component.clone();
                 self.advance();

                 if matches!(self.current(), Token::ParticleDa) {
                     self.advance();
                     
                     // Extract style from expr
                    let style = match expr {
                        Expr::Variable(s) => s,
                        _ => return Err("Expected style variable".to_string()),
                    };
                    Ok(Statement::ComponentDefine { target: target.clone(), style, component })
                 } else {
                      // Maybe it was just an assignment value that happened to mean something else?
                      // But current logic for Assignment is consume Value then expect Da.
                      // If we are here, we consumed Value, and next is Noun.
                      // Value must be Style.
                      Err("Expected 'だ' after component definition".to_string())
                 }
            }
            _ => Err("Expected 'だ', 'を', or 'な' (or Component Name)".to_string())
        }
    }

    fn parse_binary_op(&mut self) -> Result<Statement, String> {
        let target = self.parse_expression()?;

        if !matches!(self.current(), Token::ParticleNi) {
            return Err("Expected 'に'".to_string());
        }
        self.advance();

        let operand = self.current_to_expr()?;

        if !matches!(self.current(), Token::ParticleWo) {
            return Err("Expected 'を'".to_string());
        }
        self.advance();

        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();

        Ok(Statement::BinaryOp { target, operand, verb })
    }

    fn parse_binary_op_reverse(&mut self) -> Result<Statement, String> {
        // [operand] を [target] に [verb]
        let operand = self.current_to_expr()?;
        
        if !matches!(self.current(), Token::ParticleWo) {
            return Err("Expected 'を'".to_string());
        }
        self.advance();
        
        // Get target
        let target = self.parse_expression()?;
        
        // Skip "に"
        if !matches!(self.current(), Token::ParticleNi) {
            return Err("Expected 'に'".to_string());
        }
        self.advance();
        
        // Get verb
        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();
        
        Ok(Statement::BinaryOp { target, operand, verb })
    }

    fn parse_unary_or_async_op(&mut self) -> Result<Statement, String> {
        let operand = self.current_to_expr()?;

        
        let mut target: Option<String> = None;

        if !matches!(self.current(), Token::ParticleWo) {
             // Maybe it was already consumed? No.
             return Err("Expected 'を'".to_string());
        }
        self.advance(); // skip を

        // Check for Async "並列で"
        let is_async = matches!(self.current(), Token::Adverb(a) if a == "並列で");
        if is_async {
            self.advance();
        }
        
        // Check for Target "画面 に" or "画面 の 中央 に" or "[Noun] に"
        if (matches!(self.current(), Token::ScreenNoun) || matches!(self.current(), Token::Noun(_))) {
             // Lookahead for 'に' or 'の'
             if matches!(self.peek(1), Token::ParticleNi) {
                 // [Target] に
                 target = match self.current() {
                     Token::ScreenNoun => Some("Screen".to_string()),
                     Token::Noun(n) => Some(n.clone()),
                     _ => None,
                 };
                 self.advance(); // skip Target
                 self.advance(); // skip に
             } else if matches!(self.peek(1), Token::ParticleNo) {
                 // [Target] の [Modifier] に
                 // e.g. 画面 の 中央 に
                 let base = match self.current() {
                     Token::ScreenNoun => "Screen".to_string(),
                     Token::Noun(n) => n.clone(),
                     _ => "Unknown".to_string(),
                 };
                 
                 self.advance(); // skip Base
                 self.advance(); // skip の
                 
                 let modifier = match self.current() {
                     Token::Noun(n) => n.clone(),
                     _ => return Err("Expected modifier noun".to_string()),
                 };
                 self.advance(); // skip Modifier
                 
                 if matches!(self.current(), Token::ParticleNi) {
                     self.advance(); // skip に
                     target = Some(format!("{}.{}", base, modifier));
                 } else {
                     // Backtrack? Or Error?
                     // If 'に' is missing, maybe it's not a target pattern.
                     return Err("Expected 'に' after modifier".to_string());
                 }
             }
        }

        let verb = match self.current() {
            Token::Verb(v) => v.clone(),
            _ => return Err("Expected verb".to_string()),
        };
        self.advance();
        
        if let Some(tgt) = target {
             let target_expr = Expr::Variable(tgt);
             Ok(Statement::BinaryOp { target: target_expr, operand, verb })
        } else if is_async {
            Ok(Statement::AsyncOp { operand, verb })
        } else {
            Ok(Statement::UnaryOp { operand, verb })
        }
    }
    fn parse_timed_statement(&mut self) -> Result<Statement, String> {
        // [Time] 秒 後 に ... / [Time] 秒 かけて ...
        let duration = self.current_to_expr()?;
        // Expect "秒"
        if !matches!(self.current(), Token::KeywordSeconds) {
            return Err("Expected '秒'".to_string());
        }
        self.advance(); // skip 秒
        
        if matches!(self.current(), Token::KeywordAfter) {
             // Delay: [Time] 秒 後 に ... おわり
             self.advance(); // skip 後 (KeywordAfter)
             
             if !matches!(self.current(), Token::ParticleNi) {
                 return Err("Expected 'に' after '後'".to_string());
             }
             self.advance(); // skip に
             
             let body = self.parse_block_until_end()?;
             Ok(Statement::DelayStatement { duration, body })
        } else if matches!(self.current(), Token::KeywordOver) {
             // Animation: [Time] 秒 かけて [Target] の [Prop] を [Value] に する
             self.advance(); // skip かけて (KeywordOver)
             
             // [Target] の
             let target = self.parse_expression()?;
             
             if !matches!(self.current(), Token::ParticleNo) {
                 return Err("Expected 'の' after target".to_string());
             }
             self.advance(); // skip の
             
             // [Prop] を
             let property = match self.current() {
                 Token::Noun(n) => n.clone(),
                 _ => return Err("Expected property noun (色, サイズ, 影)".to_string()),
             };
             self.advance();
             
             if !matches!(self.current(), Token::ParticleWo) {
                 return Err("Expected 'を' after property".to_string());
             }
             self.advance(); // skip を
             
             // [Value] に
             let value = self.current_to_expr()?;
             
             if !matches!(self.current(), Token::ParticleNi) {
                 return Err("Expected 'に' after value".to_string());
             }
             self.advance(); // skip に
             
             // する
             if !matches!(self.current(), Token::Verb(v) if v == "する") && !matches!(self.current(), Token::KeywordChange) {
                  return Err("Expected 'する' at end of animation".to_string());
             }
             self.advance(); // skip する
             
             Ok(Statement::AnimateStatement { duration, target, property, value })
        } else {
             Err("Expected '後' (after) or 'かけて' (over) after Time".to_string())
        }
    }

    fn parse_expr_statement(&mut self) -> Result<Statement, String> {
        let expr = self.parse_expression()?;

        // Case 1: Binary (Expr に Value を Verb)
        if matches!(self.current(), Token::ParticleNi) {
            self.advance(); // skip に
            
            let operand = self.parse_expression()?;
            
            if !matches!(self.current(), Token::ParticleWo) {
                 return Err("Expected 'を'".to_string());
            }
            self.advance(); // skip を
            
            let verb = match self.current() {
                Token::Verb(v) => v.clone(),
                Token::KeywordNotify => "通知する".to_string(),
                Token::KeywordAccrue | Token::KeywordIncrease => "増やす".to_string(),
                Token::KeywordDecrease => "減らす".to_string(),
                Token::KeywordDeepen => "深くする".to_string(),
                _ => return Err("Expected verb".to_string()),
            };
            self.advance();
            
            if verb == "通知する" {
                 return Ok(Statement::Notify { target: expr, message: operand });
            }
            
            // Try to use BinaryOp if target is simple variable
            if let Expr::Variable(name) = &expr {
                 return Ok(Statement::BinaryOp { target: Expr::Variable(name.clone()), operand, verb });
            }
            
            // Otherwise use VariableUpdate (for PropertyAccess etc)
            // e.g. User.Toku に 5 を 加算する -> VariableUpdate { target, value=5, verb="増やす" }
            return Ok(Statement::VariableUpdate { 
                target: expr, 
                value: operand, 
                verb 
            });
        }
        
        // Case 2: Unary/Async (Expr を Verb)
        if matches!(self.current(), Token::ParticleWo) {
             self.advance(); // skip を
             
             // Check for Variable Update / Unary Op
             let verb = match self.current() {
                 Token::Verb(v) => v.clone(),
                 Token::KeywordIncrease | Token::KeywordAccrue => "増やす".to_string(),
                 Token::KeywordDecrease => "減らす".to_string(),
                 Token::KeywordUpdate => "更新する".to_string(),
                 Token::KeywordDeepen => "深くする".to_string(),
                 _ => return Err("Expected verb".to_string()),
             };
             self.advance();
             
             if verb == "増やす" || verb == "減らす" || verb == "深くする" {
                  return Ok(Statement::VariableUpdate {
                      target: expr,
                      value: Expr::Number(1.0),
                      verb,
                  });
             }
             
             return Ok(Statement::UnaryOp { operand: expr, verb });
        }
        
        // Case 3: Standalone Call
        if let Expr::Call { name, args } = expr {
             return Ok(Statement::ActionCall { name, args });
        }
        
        Err(format!("Unexpected token after expression: {:?}", self.current()))
    }
    
    // === AGN 2.0 Parsers ===
    
    fn parse_rule_definition(&mut self) -> Result<Statement, String> {
        self.advance(); // skip ルール
        
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected rule name".to_string()),
        };
        self.advance();
        
        // Optional {
        if matches!(self.current(), Token::LBrace) {
            self.advance();
            let body = self.parse_block_until_brace_end()?;
            // RBrace consumed by helper
            Ok(Statement::RuleDefinition { name, body })
        } else {
            let body = self.parse_block_until_end()?;
            Ok(Statement::RuleDefinition { name, body })
        }
    }
    
    fn parse_action_definition(&mut self) -> Result<Statement, String> {
        self.advance(); // skip アクション
        
        let name = match self.current() {
            Token::Noun(n) => n.clone(),
            Token::Verb(v) => v.clone(), // アクション名は動詞でもOK
            _ => return Err("Expected action name".to_string()),
        };
        self.advance();
        
        // Params: ( P1, P2 )
        let mut params = Vec::new();
        if matches!(self.current(), Token::LParen) {
            self.advance();
            loop {
                if matches!(self.current(), Token::RParen) {
                    self.advance();
                    break;
                }
                match self.current() {
                    Token::Noun(n) => params.push(n.clone()),
                    _ => return Err("Expected parameter name".to_string()),
                }
                self.advance();
                
                if matches!(self.current(), Token::Comma) {
                    self.advance();
                } else if !matches!(self.current(), Token::RParen) {
                    return Err("Expected comma or closing parenthesis".to_string());
                }
            }
        }
        
        // Optional {
        let body = if matches!(self.current(), Token::LBrace) {
            self.advance();
            self.parse_block_until_brace_end()?
        } else {
            self.parse_block_until_end()?
        };
        
        Ok(Statement::ActionDefinition { name, params, body })
    }

    // Phase 15: on Event(Type) from A to B { ... }
    fn parse_event_listener(&mut self) -> Result<Statement, String> {
        self.advance(); // skip 'on'
        self.advance(); // skip 'Event' / 'イベント' （peekで確認済み）
        
        // (Type)
        if !matches!(self.current(), Token::LParen) {
            return Err("Expected '(' after Event".to_string());
        }
        self.advance(); // skip (
        
        let event_type = match self.current() {
            Token::Noun(n) => n.clone(),
            _ => return Err("Expected event type name".to_string()),
        };
        self.advance(); // skip type
        
        if !matches!(self.current(), Token::RParen) {
            return Err("Expected ')' after event type".to_string());
        }
        self.advance(); // skip )
        
        // Optional: from A
        let mut from_var = None;
        if matches!(self.current(), Token::KeywordFrom | Token::KeywordBetween) {
             self.advance(); // skip from
             match self.current() {
                 Token::Noun(n) => {
                     from_var = Some(n.clone());
                     self.advance();
                 },
                 _ => return Err("Expected variable name after from".to_string()),
             }
        }
        
        // Optional: to B
        let mut to_var = None;
        if matches!(self.current(), Token::KeywordTo) {
             self.advance(); // skip to
             match self.current() {
                 Token::Noun(n) => {
                     to_var = Some(n.clone());
                     self.advance();
                 },
                 _ => return Err("Expected variable name after to".to_string()),
             }
        }
        
        // Block
        let body = if matches!(self.current(), Token::LBrace) {
            self.advance();
            self.parse_block_until_brace_end()?
        } else {
            return Err("Expected '{' for event body".to_string());
        };
        
        Ok(Statement::EventListener {
            event_type,
            from_var,
            to_var,
            body
        })
    }

    // Phase 10: Events
    fn parse_event_handler(&mut self) -> Result<Statement, String> {
        self.advance(); // skip 'on'
        
        let target = self.current_to_expr()?;
        
        // click / Drag / ...
        let event = if matches!(self.current(), Token::KeywordClick) {
            "click".to_string()
        } else if matches!(self.current(), Token::KeywordDrag) {
            "drag".to_string()
        } else if let Token::Verb(v) = self.current() {
            v.clone()
        } else {
             return Err("Expected event name (click, drag, etc.)".to_string());
        };
        self.advance();
        
        // Block
        let body = self.parse_block_until_end()?;
        
        Ok(Statement::EventHandler { target, event, body })
    }
    
    fn parse_block_until_brace_end(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.current(), Token::RBrace | Token::EOF) {
                if matches!(self.current(), Token::RBrace) {
                    self.advance();
                }
                break;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_english_action_command(&mut self) -> Result<Statement, String> {
        let verb = match self.current() {
            Token::KeywordIncrease => "増やす".to_string(),
            Token::KeywordDecrease => "減らす".to_string(),
            Token::KeywordUpdate => "更新する".to_string(),
            _ => return Err("Expected action verb".to_string()),
        };
        self.advance();
        
        // Target (Expression)
        let target = self.current_to_expr()?;
        
        // "by" or "to" check
        // "by" is likely Noun("by") as it is not a keyword
        let is_by = matches!(self.current(), Token::Noun(n) if n == "by");
        let is_to = matches!(self.current(), Token::KeywordTo);
        
        if is_by || is_to {
            self.advance();
        }
        
        let value = self.current_to_expr()?;
        
        Ok(Statement::VariableUpdate { target, value, verb })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    // === AGN 2.0 Tests ===

    #[test]
    fn test_parse_rule() {
        let mut lexer = Lexer::new("ルール MyRule { X は 1 だ }");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        match &program.statements[0] {
            Statement::RuleDefinition { name, body } => {
                assert_eq!(name, "MyRule");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected rule definition"),
        }
    }

    #[test]
    fn test_property_update() {
        // User.Toku を 増やす
        let mut lexer = Lexer::new("User.Toku を 増やす");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        match &program.statements[0] {
            Statement::VariableUpdate { target, value, verb } => {
                // target should be PropertyAccess
                match target {
                    Expr::PropertyAccess { property, .. } => assert_eq!(property, "Toku"),
                    _ => panic!("Expected propery access target"),
                }
                // value 1.0 (default increment)
                match value {
                    Expr::Number(n) => assert_eq!(*n, 1.0),
                    _ => panic!("Expected number value"),
                }
                assert_eq!(verb, "増やす");
            }
            _ => panic!("Expected variable update"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        let mut lexer = Lexer::new("X は 10 だ");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { target, value } => {
                if let Expr::Variable(name) = target {
                    assert_eq!(name, "X");
                } else {
                    panic!("Expected Variable target");
                }
                match value {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected assignment"),
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let mut lexer = Lexer::new("X に 5 を 足す");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::BinaryOp { target, operand, verb } => {
                if let Expr::Variable(name) = target {
                    assert_eq!(name, "X");
                } else {
                    panic!("Expected Variable target");
                }
                assert_eq!(verb, "足す");
                match operand {
                    Expr::Number(n) => assert_eq!(*n, 5.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected binary op"),
        }
    }

    #[test]
    fn test_parse_async_op() {
        let mut lexer = Lexer::new("X を 並列で 表示する");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::AsyncOp { operand, verb } => {
                assert_eq!(verb, "表示する");
                match operand {
                    Expr::Variable(name) => assert_eq!(name, "X"),
                    _ => panic!("Expected variable"),
                }
            }
            _ => panic!("Expected async op"),
        }
    }

    // === Phase 4 Tests ===
    
    #[test]
    fn test_parse_english_show() {
        let mut lexer = Lexer::new("show X");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::UnaryOp { operand, verb } => {
                assert_eq!(verb, "表示する");
                match operand {
                    Expr::Variable(name) => assert_eq!(name, "X"),
                    _ => panic!("Expected variable"),
                }
            }
            _ => panic!("Expected unary op"),
        }
    }

    #[test]
    fn test_parse_english_add_to() {
        let mut lexer = Lexer::new("add 5 to X");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::BinaryOp { target, operand, verb } => {
                if let Expr::Variable(name) = target {
                    assert_eq!(name, "X");
                } else {
                    panic!("Expected Variable target");
                }
                assert_eq!(verb, "足す");
                match operand {
                    Expr::Number(n) => assert_eq!(*n, 5.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected binary op"),
        }
    }

    #[test]
    fn test_parse_english_if() {
        let mut lexer = Lexer::new("if X equals 5 then show X end");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::IfStatement { condition, then_block, else_block } => {
                match condition {
                    Condition::Equals(_, _) => {}
                    _ => panic!("Expected equals condition"),
                }
                assert_eq!(then_block.len(), 1);
                assert!(else_block.is_none());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_parse_english_repeat() {
        let mut lexer = Lexer::new("repeat 10 times add 1 to X end");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::RepeatStatement { count, body } => {
                match count {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected repeat statement"),
        }
    }

    #[test]
    fn test_parse_let() {
        let mut lexer = Lexer::new("let X is 10");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { target, value } => {
                if let Expr::Variable(name) = target {
                    assert_eq!(name, "X");
                } else {
                    panic!("Expected Variable target");
                }
                match value {
                    Expr::Number(n) => assert_eq!(*n, 10.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected assignment"),
        }
    }
}
