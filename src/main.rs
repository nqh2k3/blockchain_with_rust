mod p2p;

use core::error;
use std::{collections::HashSet, thread::spawn, time::Duration};

use chrono::format;
use libp2p::{core::transport::upgrade, futures::channel::mpsc, identity::ed25519::Keypair, mplex, noise::NoiseConfig, swarm::{self, protocols_handler::multi::Info, SwarmBuilder}, tcp::TokioTcpConfig, Swarm};
use log::{info, warn};
use p2p::BLOCK_TOPIC;
use serde_json::json;
use sha2::Sha256;
use tokio::time::sleep;


pub struct App {
    pub blocks: Vec,
}

pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data:String,
    pub nonce: u64,
}

const DIFFULTY_PREFIX: &str = "00";

fn hash_to_binary_representation(hash: &[u8],) -> String{
    let mut res: String = String::default();
    for c in hash {
        res.push(&format!("{:b}",c));
    }
    res
}

impl App {
    fn new () -> Self {Self {blocks: vec![]}}
    fn genesis (&mut self){
        let genesis_block = Block{
            id: 0,
            timestamp: Utc::now ().timestamp(),
            previous_hash: String::from("Genesis"),
            data: String::from("genesis!"),
            nonce:2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),

        };
        self.blocks.push(genesis_block);
    }

    fn try_add_block (&mut self, block: Block){
        let latest_block = self.blocks.last().expect("There is at latest one block");
        if self.is_block_valid(&block, latest_block){
            self.blocks.push(block);
        }else {
            error!("Could not add block - Invalid");
        }
    }

    fn is_block_valid (&self, block: &Block, previous_block: &Block) -> bool{

        if block.previous_hash != previous_block.hash{
            warn!("Block with id: {} has wrong previous hash", block.id);
            return false;
        }else if !hash_to_binary_representation(&hex::decode(&block.hash).expect_err("Can decode from here"),)
        .starts_with(DIFFULTY_PREFIX)
         {
            warn!("Block with id: {} has invalid difficulty", block.id);
            return  false;
        }else if block.id != previous_block.id + 1 {
            warn!("Block with id: {} is not the next after latest: {}", block.id, previous_block.id);
            return false;
        }else if  hex::encode(caculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
            
        )) != block.hash{
           warn!("Block with id:{} has invalid hash", block.id);
           return false;
            
        }
        true
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool{
        for i in 0..chain.len(){
            if i == 0{ continue;}
        }

        let first = chain.get(i - 1).expect("Has to exist");
        let second = chain.get(i).expect("Has to exist");
        if !self.is_block_valild(second, first){
            return false;
        }
        true
    }

    fn choose_chain(&mut self, local: Vec, remote: Vec) -> Vec{
        
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);
        
        if is_local_valid && is_remote_valid{
            if local.len() >= remote.len() {
                local
            }else {
                remote
            }
        }else if is_remote_valid && !is_local_valid {
            remote
        }else if !is_remote_valid && is_local_valid {
            local
        }else {
            panic!("Local and remote chains are both invalid");
        }
    }

}

impl Block {
    pub fn new (id:u64, previous_hash: String, data:String) -> Self{
        let now = Utc::now();
        let (nonce, hash) = mine_block(id, now.timestamp(), &previous_hash, &data);
        Self{
            id,
            hash,
            timestamp: now.timestamp(),
            data,
            nonce,
            previous_hash,
        }
    }
}
 fn mine_block(id: u64, timestamp: i64, previous_hash:&str, data: &str) ->(u64,String){
    Info!("Mining block ...");
    let mut nonce = 0;
    loop{
        if nonce % 100000 ==0{
            info!("Nonce: {}", nonce);
        }

    }
    let hash = caculate_hash(id, timestamp, previous_hash, data, nonce);
    let binary_hash = hash_to_binary_representation(&hash);
    if binary_hash.starts_with(DIFFULTY_PREFIX){
        info!("Mined! nonce: {}, hash: {}, binary hash:{}",
        nonce,
        hex::encode(&hash),
        binary_hash,
    );
    return (nonce, hex::encode(hash));
    }
    nonce += 1;

 }

 fn caculate_hash(id:u64, timestamp: i64,previous_hash:&str,data: &str, nonce:u64) -> Vec<u8>{
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp":timestamp,
        "nonce": nonce
    });

    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    hasher.finalize().as_slice().to_owned()



 }
 
 #[tokio::main]

async  fn main() {
    pretty_env_logger::init();
    info!("Peer id: {}", p2p::PEER_ID.clone());
    let (respone_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::new()
    .into_authentic(&p2p::KEYS)
    .expect("Can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let beheviour = p2p::AppBehaviour::new(App::new(), respone_sender, init_sender.clone()).await;
    let mut swarm = SwarmBuilder::new(transp, beheviour,*p2p::PEER_ID )
        .excutor(Box::new(|fut|{
            spawn(fut);
        }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(&mut swarm, "/ip/0.0.0.0/tcp/0"
        .parse()
        .expect("Can get a local socket"),
    )
    .expect("Swarm can be started");
    
    spawn(async move{
        sleep(Duration::from_secs(1)).await;
        info!("Sending init event");
        init_sender.sen(true).expect("Can send init event");
    });


    loop{
        let evt ={
            select! {
                line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("Can get line").expect("Can read line from stdin"))),
                respone = response_rcv.recv() =>{
                    Some(p2p::EventType::LocalChainRespone(respone.expect("Respone exists")))
                },
                _init = init_rcv.recv() =>{
                    Some(p2p::EventType::Init)
                }

                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }

        };

        if let Some(event) = evt{
            match event{
                p2p::EvenType::Init =>{
                    let peers = p2p::get_lisr_peers(&Swarm);
                    swarm.behaviour_mut().app.genesis();
                    
                    info!("Connect nodes: {}", peers.len());
                    if !peers.is_empty(){
                        let req = p2p::LocalChainRequest{
                            from_peer_id: peers
                            .iter()
                            .last()
                            .expect("At last one peer")
                            .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("Can jsonfy request");
                        swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());

                        
                    }
                }
                p2p::EvenType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("Can jsonfy respone");
                    swarm
                    .behaviour_mut()
                    .floodsub
                    .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                p2p::EvenType::Input(line) => match  line.as_str() {
                    "ls p" => p2p::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => p2p::handle_print+chain(&swarm),
                    cmd if cmd.starts_with("create b") => p2p::handle_create_block(cmd, &mut swarm),
                    _ => error("Unknown command"),
                    
                },
            }
        }
    }

    pub fn get_lisr_peers(swarm: &Swarm) -> Vec {
        info!("Discover");
        let nodes = swarm.behaviour().mdns.discovered_nodes();
        let mut unique_peers = HashSet::new();
        for peer in nodes {
            unique_peers.insert(peer);
        }
        unique_peers.iter().map(|p| p.to_string().collect())
    }

    pub fn handle_print_peers(swarm: &Swarm){
        let peers = get_lisr_peers(swarm);
        peers.iter().for_each(|p| info!("{}", p));
    }

    pub fn handle_create_block(cmd: &str, swarm: &mut Swarm){
        if let Some(data) = cmd.strip_prefix("create b"){
            let behaviour = swarm.behaviour_mut();
            let latest_block = behaviour
                            .app
                            .blocks
                            .last()
                            .expect("There is last one block");

            let block = Block::new(
                latest_block.id + 1,
                latest_block.hash.clone(),
                data.to_owned(),
            );

            let json = serde_json::to_string(&block).expect("Can jsonfy request");
            behaviour.app.blocks.push(block);
            info!("Broadcasting new block");
            behaviour
                .floodsub
                .publish(BLOCK_TOPIC.clone(), json.as_bytes());            
        }
    }

}
