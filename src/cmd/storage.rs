// src/cmd/storage.rs — Storage bucket command handlers.

use crate::api::types::{CreateBucketRequest, ExternalBucketRequest};
use crate::cli::*;
use crate::ctx::AppContext;
use crate::output::{self, table};

pub async fn handle(ctx: &AppContext, cmd: StorageCommand) -> anyhow::Result<()> {
    ctx.require_api_key()?;
    match cmd {
        StorageCommand::Bucket(args) => bucket(ctx, args.command).await,
    }
}

async fn bucket(ctx: &AppContext, cmd: StorageBucketCommand) -> anyhow::Result<()> {
    match cmd {
        StorageBucketCommand::Create(args) => {
            if args.name.trim().is_empty() {
                anyhow::bail!("bucket name is required");
            }
            let req = CreateBucketRequest {
                name: args.name,
                region: args.region,
                project_id: args.project_id,
                external: None,
            };
            let bucket = ctx.api.storage().create_bucket(&req).await?;
            output::print_or_json(ctx.output, &bucket, || {
                println!("{}", table::bucket_detail_table(&bucket));
            });
        }
        StorageBucketCommand::Connect(args) => {
            if args.name.trim().is_empty() {
                anyhow::bail!("bucket name is required");
            }
            if args.bucket.trim().is_empty() {
                anyhow::bail!("provider bucket is required");
            }
            if !matches!(args.provider.as_str(), "s3" | "s3_compatible") {
                anyhow::bail!("provider must be 's3' or 's3_compatible'");
            }
            if !matches!(args.auth_mode.as_str(), "static" | "oidc") {
                anyhow::bail!("auth mode must be 'static' or 'oidc'");
            }
            match (args.provider.as_str(), args.endpoint.as_deref()) {
                ("s3", Some(_)) => {
                    anyhow::bail!("--endpoint is for s3_compatible buckets only");
                }
                ("s3_compatible", None) => {
                    anyhow::bail!("s3_compatible buckets need --endpoint https://...");
                }
                (_, Some(ep)) if !ep.to_ascii_lowercase().starts_with("https://") => {
                    anyhow::bail!("endpoint must be an https:// URL");
                }
                _ => {}
            }
            match args.auth_mode.as_str() {
                "static" => {
                    if args.secret_provider_id.is_none()
                        || args.access_key_secret.is_none()
                        || args.secret_key_secret.is_none()
                    {
                        anyhow::bail!(
                            "static auth needs --secret-provider-id, --access-key-secret, and --secret-key-secret"
                        );
                    }
                }
                "oidc" => {
                    if args.provider != "s3" {
                        anyhow::bail!("oidc auth is s3-only");
                    }
                    if args.role_arn.is_none() || args.external_id.is_none() {
                        anyhow::bail!("oidc auth needs --role-arn and --external-id");
                    }
                }
                _ => {}
            }
            if let Some(ref p) = args.prefix {
                if p.starts_with('/') || p.split('/').any(|s| s == "..") {
                    anyhow::bail!("prefix must be relative — no leading '/' or '..' segments");
                }
            }
            let req = CreateBucketRequest {
                name: args.name,
                region: None,
                project_id: args.project_id,
                external: Some(ExternalBucketRequest {
                    provider: args.provider,
                    bucket: args.bucket,
                    auth_mode: args.auth_mode,
                    prefix: args.prefix,
                    region: args.region,
                    endpoint: args.endpoint,
                    path_style: if args.path_style { Some(true) } else { None },
                    secret_provider_id: args.secret_provider_id,
                    access_key_secret: args.access_key_secret,
                    secret_key_secret: args.secret_key_secret,
                    session_token_secret: args.session_token_secret,
                    role_arn: args.role_arn,
                    external_id: args.external_id,
                }),
            };
            let bucket = ctx.api.storage().create_bucket(&req).await?;
            output::print_or_json(ctx.output, &bucket, || {
                println!("{}", table::bucket_detail_table(&bucket));
            });
        }
        StorageBucketCommand::List(args) => {
            let result = ctx
                .api
                .storage()
                .list_buckets(args.project_id.as_deref())
                .await?;
            output::print_or_json(ctx.output, &result, || {
                println!("{}", table::bucket_table(&result.buckets));
                println!("  ({} total)", result.total);
            });
        }
        StorageBucketCommand::Get(args) => {
            super::validate_resource_id(&args.id, "bucket")?;
            let bucket = ctx.api.storage().get_bucket(&args.id).await?;
            output::print_or_json(ctx.output, &bucket, || {
                println!("{}", table::bucket_detail_table(&bucket));
            });
        }
        StorageBucketCommand::Verify(args) => {
            super::validate_resource_id(&args.id, "bucket")?;
            let bucket = ctx.api.storage().verify_bucket(&args.id).await?;
            output::print_or_json(ctx.output, &bucket, || {
                println!("{}", table::bucket_detail_table(&bucket));
            });
        }
        StorageBucketCommand::Delete(args) => {
            super::validate_resource_id(&args.id, "bucket")?;
            ctx.api.storage().delete_bucket(&args.id).await?;
            output::print_or_json(ctx.output, &true, || {
                println!("Deleted {}", args.id);
            });
        }
    }
    Ok(())
}
