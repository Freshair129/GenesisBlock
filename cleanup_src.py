import os

path = 'src/lib.rs'
content = open(path, encoding='utf-8').read()

# 1. Update ConsensusProposal definition
old_cp = """#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusProposal {
    pub proposal_id: String,
    pub event: Event,
    pub signature: Vec<u8>,
    pub votes: HashMap<String, bool>, // PeerID -> Vote
}"""

new_cp = """#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusProposal {
    pub proposal_id: String,
    pub signed_event: SignedEvent,
    pub votes: HashMap<String, bool>, // PeerID -> Vote
    pub quorum_signatures: HashMap<String, Vec<u8>>, // PeerID -> Signature
}"""

if old_cp in content:
    content = content.replace(old_cp, new_cp)

# 2. Add execute_batch to GenesisDatabase if missing
new_gb_impl = """    #[napi] pub async fn bulk_add_nodes(&self, inputs: Vec<NodeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_nodes(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }
    #[napi] pub async fn execute_batch(&self, input: BatchInput) -> Result<BatchOutput> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.execute_batch(input)).await.map_err(|e| Error::from_reason(e.to_string()))? }"""

if 'execute_batch' not in content:
    content = content.replace('pub async fn bulk_add_edges(&self, inputs: Vec<EdgeInput>) -> Result<()> { let i = Arc::clone(&self.inner); tokio::task::spawn_blocking(move || i.bulk_add_edges(inputs)).await.map_err(|e| Error::from_reason(e.to_string()))? }', new_gb_impl)

# 3. Cleanup EOF duplicates
search_str = '#[napi] pub fn engine_name_sync() -> String { "genesis-block".to_string() }\n#[napi] pub fn schema_version_sync() -> u32 { SCHEMA_VERSION }'
if content.count(search_str) > 1:
    content = content[:content.rfind(search_str)] + search_str

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
