use std::{error::Error, time::Duration, num::NonZeroU32};

use futures::stream::StreamExt;
use azure_storage::core::prelude::*;
use azure_storage_blobs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    //  kubectl --namespace=az-mocks port-forward deployment/azurite-depl 10000:10000
    let blob_endpoint = r#"BlobEndpoint=http://127.0.0.1:10000/devstoreaccount1;"#;
    let connection_string = format!("UseDevelopmentStorage=true;{}", blob_endpoint);

    let container_name = "test";

    let http_client = azure_core::new_http_client();

    let storage_client = StorageAccountClient::new_connection_string(http_client.clone(), &connection_string)?.as_storage_client();
    let container_client = storage_client.as_container_client(container_name);
    let blob_service = storage_client.as_blob_service_client();

    let iv = blob_service.list_containers().execute().await?;

    if iv
        .incomplete_vector
        .iter()
        .any(|item| item.name == container_name)
    {
        panic!("The specified container must not exists!");
    }

    // create the container
    container_client
        .create()
        .public_access(PublicAccess::None)
        .timeout(Duration::from_secs(100))
        .execute()
        .await?;
    println!("Container {} created", container_name);

    // create 10 blobs
    for i in 0..10u8 {
        container_client
            .as_blob_client(format!("blob{}.txt", i))
            .put_block_blob("somedata")
            .content_type("text/plain")
            .execute()
            .await?;
        println!("\tAdded blob {}", i);
    }

    let max_results = NonZeroU32::new(3).unwrap();
    let iv = container_client
        .list_blobs()
        .max_results(max_results)
        .execute()
        .await?;

    println!("List blob returned {} blobs.", iv.blobs.blobs.len());
    for cont in iv.blobs.blobs.iter() {
        println!("\t{}\t{} bytes", cont.name, cont.properties.content_length);
    }

    let mut stream = Box::pin(
        container_client
            .list_blobs()
            .max_results(max_results)
            .stream(),
    );

    let mut cnt: i32 = 0;
    while let Some(value) = stream.next().await {
        let len = value?.blobs.blobs.len();
        println!("received {} blobs", len);
        match cnt {
            0 | 1 | 2 => assert_eq!(len, 3),
            3 => assert_eq!(len, 1),
            _ => panic!("more than 10 entries??"),
        }
        cnt += 1;
    }

    container_client.delete().execute().await?;
    println!("Container {} deleted", container_name);

    Ok(())

}
