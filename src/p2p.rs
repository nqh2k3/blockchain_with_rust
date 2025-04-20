use libp2p::{floodsub::{Floodsub, FloodsubEvent}, identity::error, mdns::{Mdns, MdnsEvent}, swarm::{NetworkBehaviour, NetworkBehaviourEventProcess}, PeerId};
use log::info;
use tokio::sync::mpsc;

use crate::App;

pub static KEYS: Lazy = Lazy::new(identiy::Keypair::generate_end25519);
pub static PEER_ID: Lazy = Lazy::new(|| PeerID::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy = Lazy::new(|| Topic::new("blocks"));

#[derive(Debug, Serialize, Deserialize)]
 pub struct ChainRespone{
    pub blocks: Vec,
    pub receiver:String,
 }

 #[derive(Debug, Serialize, Deserialize)]
 pub struct LocalChainRequest{
    pub from_peer_id:String,
 }
 
 pub enum EvenType{
    LocalChainResponse(ChainRespone),
    Input(String),
    Init,
 }

 #[derive(NetworkBehaviour)]
 pub struct AppBehaviour{
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behavious(ignore)]
    pub response_sender: mpsc::UnboundedSender,
    #[behaviour(ignore)]
    pub app:App,
    init_sender: mpsc::UnboundedSender<_>,
 }


 impl AppBehaviour {
     pub async fn new(
        app:App,
        response_sender: mpsc::UnboundedSender,
        init_sender:mpsc::UnboundedSender,

     ) -> Self{
        let mut behaviour = Self{
            app,
            floodsub: Floodsub::new(*PeerId),
            mdns: Mdns::new(Default::default())
            .await
            .expect("Can create mdns"),
            response_sender,
            init_sender,
        };

        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        behaviour
     }


 }

 impl NetworkBehaviourEventProcess <MdnsEvent> for AppBehaviour {
     fn inject_event(&mut self, event:MdnsEvent){
        match event{
            MdnsEvent::Discovered(discoverd_list) =>{
                for (peer, _addr) in discoverd_list{
                    self.floodsub.add_node_to_partial_view(peer);
                }
                
            }
            MdnsEvent::Expired(expired_list) =>{
                for (peer, _addr) in expired_list{
                    if !self.mdns.has_node(&peer){
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
     }
 }

 impl  NetworkBehaviourEventProcess for AppBehaviour{
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event{
            if let Ok(resp) = serde_json::from_slice(&msg.data){
                if resp.receiver == PEER_ID.to_string(){
                    info!("Respone from {}," msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}",r));

                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(),resp.blocks);
                }
            }
        }else if let Ok(resp) = serde_json::from_slice(&msg.data) {
            info!("Sending local chain to {}", msg.source.to_string());
            if PeerId.to_string() == peer_id {
                if let Err(e) = self.response_sender.send(ChainRespone{
                    blocks:self.app.blocks.clone(),
                    receiver: msg.source.to_string(),

                }){
                    error("Error sending via channel, {}", e);
                }
            }else if let Ok(block) = serde_json::from_slice(&msg.data) {
                info!("Received new block from {}", msg.source.to_string());
                self.app.try_add_block(block);
            }
        }
    }
 }