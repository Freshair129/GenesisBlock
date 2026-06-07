import os

path = 'src/lib.rs'
content = open(path, encoding='utf-8').read()

new_propose = """    pub fn propose_consensus(&self, event: Event, signature: Vec<u8>) -> Result<String> {
        let proposal_id = Uuid::new_v4().to_string();
        let signed_event = SignedEvent {
            event,
            signature,
            signer_peer_id: self.local_peer_id.clone(),
        };
        let proposal = ConsensusProposal {
            proposal_id: proposal_id.clone(),
            signed_event,
            votes: HashMap::new(),
            quorum_signatures: HashMap::new(),
        };
        self.proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }"""

new_submit = """    pub fn submit_vote(&self, proposal_id: String, peer_id: String, approve: bool) -> Result<bool> {
        if let Some(mut proposal_ref) = self.proposals.get_mut(&proposal_id) {
            let proposal = proposal_ref.value_mut();
            proposal.votes.insert(peer_id.clone(), approve);
            
            let approvals = proposal.votes.values().filter(|&&v| v).count();
            
            if approvals > (self.peers.len() / 2) {
                let signed_event = &proposal.signed_event;
                match &signed_event.event {
                    Event::Node(n) => {
                        let mut n_axiom = n.clone();
                        if !n_axiom.labels.contains(&"MASTER".to_string()) { 
                            n_axiom.labels.push("MASTER".to_string()); 
                        }
                        let u32_id = self.get_or_intern_id(&n_axiom.id);
                        self.nodes.insert(u32_id, n_axiom.clone());
                        self.persist_signed(SignedEvent {
                            event: Event::Node(n_axiom),
                            signature: signed_event.signature.clone(),
                            signer_peer_id: signed_event.signer_peer_id.clone(),
                        })?;
                    }
                    Event::Edge(e) => {
                        let u32_id = self.get_or_intern_id(&e.id);
                        self.edges.insert(u32_id, e.clone());
                        self.persist_signed(signed_event.clone())?;
                    }
                    Event::Batch(events) => {
                        for e in events {
                            match e {
                                Event::Node(n) => {
                                    let mut n_axiom = n.clone();
                                    if !n_axiom.labels.contains(&"MASTER".to_string()) { n_axiom.labels.push("MASTER".to_string()); }
                                    let u32_id = self.get_or_intern_id(&n_axiom.id);
                                    self.nodes.insert(u32_id, n_axiom);
                                }
                                Event::Edge(edge) => {
                                    let u32_id = self.get_or_intern_id(&edge.id);
                                    self.edges.insert(u32_id, edge.clone());
                                }
                                _ => {}
                            }
                        }
                        self.persist_signed(signed_event.clone())?;
                    }
                }
                return Ok(true);
            }
        }
        Ok(false)
    }"""

# Find and replace propose_consensus
start = content.find('pub fn propose_consensus')
end = content.find('    pub fn submit_vote')
if start != -1 and end != -1:
    content = content[:start] + new_propose + "\n\n" + content[end:]

# Find and replace submit_vote
start = content.find('pub fn submit_vote')
end = content.find('    pub fn calculate_sc')
if start != -1 and end != -1:
    content = content[:start] + new_submit + "\n\n" + content[end:]

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
