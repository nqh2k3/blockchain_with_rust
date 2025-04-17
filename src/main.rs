use chrono::format;


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

    fn is_block_valid (&self, block: &Block, previous_block: &Block){

    }
}

fn main() {
    println!("Hello, world!");
}
