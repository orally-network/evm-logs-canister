use ic_cdk_macros::{query, update};
use ic_utils::{
  api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
  get_information, update_information,
};

#[query(name = "getCanistergeekInformation")]
pub async fn get_canistergeek_information(request: GetInformationRequest) -> GetInformationResponse<'static> {
  get_information(request)
}

#[update(name = "updateCanistergeekInformation")]
pub async fn update_canistergeek_information(request: UpdateInformationRequest) {
  update_information(request);
}
