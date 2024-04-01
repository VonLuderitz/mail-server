/*
 * Copyright (c) 2023 Stalwart Labs Ltd.
 *
 * This file is part of Stalwart Mail Server.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of
 * the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 * in the LICENSE file at the top-level directory of this distribution.
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * You can be released from the requirements of the AGPLv3 license by
 * purchasing a commercial license. Please contact licensing@stalw.art
 * for more details.
*/

pub mod domain;
pub mod principal;
pub mod queue;
pub mod reload;
pub mod report;
pub mod settings;
pub mod stores;

use std::{borrow::Cow, sync::Arc};

use http_body_util::combinators::BoxBody;
use hyper::{body::Bytes, Method};
use jmap_proto::error::request::RequestError;
use serde::Serialize;
use serde_json::json;

use crate::{auth::{oauth::OAuthCodeRequest, AccessToken}, JMAP};

use super::{http::ToHttpResponse, HttpRequest, JsonResponse};

#[derive(Serialize)]
#[serde(tag = "error")]
pub enum ManagementApiError {
    FieldAlreadyExists {
        field: Cow<'static, str>,
        value: Cow<'static, str>,
    },
    FieldMissing {
        field: Cow<'static, str>,
    },
    NotFound {
        item: Cow<'static, str>,
    },
    Unsupported {
        details: Cow<'static, str>,
    },
    AssertFailed,
    Other {
        details: Cow<'static, str>,
    },
    UnsupportedDirectoryOperation {
        class: Cow<'static, str>,
    },
}

impl JMAP {
    pub async fn handle_api_manage_request(
        &self,
        req: &HttpRequest,
        body: Option<Vec<u8>>,
        access_token: Arc<AccessToken>,
    ) -> hyper::Response<BoxBody<Bytes, hyper::Error>> {
        let path = req.uri().path().split('/').skip(2).collect::<Vec<_>>();
        let is_superuser = access_token.is_super_user();

        match path.first().copied().unwrap_or_default() {
            "principal" if is_superuser => {
                self.handle_manage_principal(req, path, body)
                    .await
            }
            "domain" if is_superuser => {
                self.handle_manage_domain(req, path)
                    .await
            }
            "store" if is_superuser => {
                self.handle_manage_store(req, path,)
                    .await
            }
            "reload" if is_superuser => {
                self.handle_manage_reload(req, path)
                    .await
            }
            "settings" if is_superuser => {
                self.handle_manage_settings(req, path, body)
                    .await
            }
            "queue" if is_superuser => {
                self.handle_manage_queue(req, path)
                    .await
            }
            "reports" if is_superuser => {
                self.handle_manage_reports(req, path)
                    .await
            }
            "oauth" => {
                match serde_json::from_slice::<OAuthCodeRequest>(body.as_deref().unwrap_or_default()) {
                    Ok(request) => {
                        JsonResponse::new(json!({
                            "data": {
                                "code": self.issue_client_code(&access_token, request.client_id, request.redirect_uri),
                                "is_admin": access_token.is_super_user(),
                            },
                        }))
                        .into_http_response()
    
                    },
                    Err(err) => err.into_http_response(),
                }
            }
            "crypto" => match *req.method() {
                Method::POST => self.handle_crypto_post(access_token, body).await,
                Method::GET => self.handle_crypto_get(access_token).await,
                _ => RequestError::not_found().into_http_response(),
            },
            "password" => match *req.method() {
                Method::POST => self.handle_change_password(req, access_token, body).await,
                _ => RequestError::not_found().into_http_response(),
            },
            _ => RequestError::not_found().into_http_response(),
        }
    }
}


impl ToHttpResponse for ManagementApiError {
    fn into_http_response(self) -> super::HttpResponse {
        JsonResponse::new(self).into_http_response()
    }
}

impl From<Cow<'static, str>> for ManagementApiError {
    fn from(details: Cow<'static, str>) -> Self {
        ManagementApiError::Other { details }
    }
}

impl From<String> for ManagementApiError {
    fn from(details: String) -> Self {
        ManagementApiError::Other { details: details.into() }
    }
}
