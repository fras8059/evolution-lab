use utoipa::OpenApi;

pub mod v1;

#[derive(OpenApi)]
#[openapi(
    paths(
        v1::run,
    ),
    components(schemas(v1::Parameters)),
    tags(
            (name = "run", description = "Run management endpoints.")
        ),
    )]
pub(super) struct ApiDoc;
