# ai-gateway-rs

Rustで書かれた、AI APIを呼ぶだけの内部向け基盤（AI Gateway）。

## なにこれ

サービスA, B, C... から同じAI関連のAPI（Google AI Studio / Groq / OpenRouter など）を叩く処理を、
このゲートウェイ1つに集約して中継するAPI鯖。

- 各サービスはこのゲートウェイだけを叩けば良い
- プロバイダ追加・差し替えはここだけで完結
- リクエスト/レスポンスのログはまとめてここから出力

## 背景

- 各サービスにプロバイダのAPIキーをばら撒きたくない（鍵管理を1箇所に）
- 上流APIの破壊的変更があっても、ラッパー層をここで吸収すれば各サービス側は無傷
- k3sに乗せる予定

## 対応プロバイダ

いずれもAPIキー1本で叩けるREST APIから着手。

- [x] Google AI Studio (Gemini API)
- [x] Groq
- [x] OpenRouter
- [ ] (将来) Vertex AI など ADC 認証系

## 技術スタック

| 領域 | 採用 |
|---|---|
| 言語 | Rust (stable) |
| Webフレームワーク | [axum](https://docs.rs/axum) |
| 非同期ランタイム | [tokio](https://tokio.rs) |
| HTTPクライアント | [reqwest](https://docs.rs/reqwest) |
| OpenAPI / Swagger UI | [utoipa](https://github.com/juhaku/utoipa) + utoipa-swagger-ui |
| シリアライズ | serde / serde_json |
| エラー | thiserror（lib層）/ anyhow（bin層） |
| 設定読み込み | dotenvy + figment(or config) |
| ロギング/トレース | tracing + tracing-subscriber（JSON出力） |
| コンテナ | docker / docker compose（開発用）, k3s（本番） |

DBは使わない（利用ログ・メトリクスはk3s側のLoki/Grafanaで集約）。

## アーキテクチャ（DDD / クリーンアーキ）

Cargoワークスペースで層ごとにcrateを切る。依存方向は `presentation → application → domain ← infrastructure`。

```
ai-gateway-rs/
├── Cargo.toml                # workspace
├── crates/
│   ├── domain/               # Entity / Value Object / Repository trait（依存ゼロ）
│   ├── application/          # ユースケース / 出力ポート（AiProvider trait など）
│   ├── infrastructure/       # 各AIプロバイダClient実装 / 設定読み込み
│   └── presentation/         # axumハンドラ / ルーティング / OpenAPI / 認証middleware
└── apps/
    └── server/               # 実行バイナリ（DI組み立て & 起動）
```

新しいAIプロバイダを追加するときは:

1. `application/port` の `AiProvider` trait を実装した型を `infrastructure/provider/<name>.rs` に追加
2. `apps/server` のDI部に登録
3. 必要ならOpenAPIスキーマを更新

## 認証

- **インターネット非公開**。k3sクラスタ内のサービスPodからのみ叩かれる前提。
- ゲートウェイへの認証は **APIキー方式**。`X-API-Key: <key>` ヘッダで判定するmiddlewareが動作中。
- 各上流プロバイダのAPIキーはこのゲートウェイの環境変数（k3s Secret）でのみ保持。

> TODO: 単一鍵 / サービス別鍵 / ローテーション運用 を決める

## エンドポイント

| Method | Path | 用途 | 状態 |
|---|---|---|:-:|
| POST | `/v1/chat/completions` | 統一インターフェースで chat 推論（テキスト/画像入力対応） | ✅ |
| GET  | `/v1/providers` | 利用可能プロバイダ一覧 | ✅ |
| GET  | `/healthz` | liveness | ✅ |
| GET  | `/readyz` | readiness | ✅ |
| GET  | `/swagger-ui` | OpenAPI ドキュメント | ✅ |

### 画像入力（マルチモーダル）

`messages[].content` は **文字列または配列** を受け付ける（OpenAI互換）。

```json
{
  "provider": "groq",
  "model": "meta-llama/llama-4-scout-17b-16e-instruct",
  "messages": [
    {
      "role": "user",
      "content": [
        { "type": "text", "text": "このレシートをJSONに" },
        { "type": "image_url", "image_url": { "url": "data:image/jpeg;base64,/9j/4AAQ..." } }
      ]
    }
  ]
}
```

注意点:

- **モデル能力チェックは呼び出し側責任**（テキスト専用モデルに画像を投げると上流が400を返す、それをそのまま中継）
- **Google AI Studio (Gemini) は `data:` URL のみ対応**（http(s) URLを送ると `InvalidRequest` で400）
- リクエストbodyの上限は **16MB**（数MB級の画像を想定）

## セットアップ（開発）

```bash
# .env をコピー
cp .env.example .env
# 必要なAPIキーを書き込む

# 依存取得 & 起動
cargo run -p server
```

または

```bash
docker compose up --build
```

## 環境変数

| 変数 | 用途 |
|---|---|
| `GATEWAY_BIND` | バインドアドレス（例: `0.0.0.0:8080`） |
| `GATEWAY_API_KEYS` | このゲートウェイ自身の受け入れAPIキー（カンマ区切り） |
| `GOOGLE_AI_STUDIO_API_KEY` | Google AI Studio (Gemini) のキー |
| `GROQ_API_KEY` | Groq のキー（後で） |
| `OPENROUTER_API_KEY` | OpenRouter のキー（後で） |
| `RUST_LOG` | ログレベル（例: `info,ai_gateway=debug`） |

## ドキュメント

- [CI/CD と ブランチ運用](docs/ci-cd.md) — GitHub Actions / GHCR / リリースフロー

## TODO

<!-- 達成済みのため一旦空 -->

