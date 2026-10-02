use qdrant_client::Qdrant;

fn check_type() {
    let client = Qdrant::from_url("http://localhost:6334").build();
}
