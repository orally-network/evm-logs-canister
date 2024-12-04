use std::str::FromStr;

use candid::CandidType;
use ic_web3_rs::{
    ethabi::Token,
    types::{H160, U256},
};
use serde::{Deserialize, Serialize};


#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SwapEventData {
    pub tx_hash: String,
    pub sender: SolidityToken,
    pub receiver: SolidityToken,
    pub amount0: SolidityToken,
    pub amount1: SolidityToken,
    pub sqrt_price_x96: SolidityToken,
    pub liquidity: SolidityToken,
    pub tick: SolidityToken,
}


/// SolidityToken is a representation of a web3_rs Token, but with CandidType support
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum SolidityToken {
    Address(String),
    FixedBytes(ic_web3_rs::ethabi::FixedBytes),
    Bytes(ic_web3_rs::ethabi::Bytes),
    Int(String),
    Uint(String),
    Bool(bool),
    String(String),
    FixedArray(Vec<SolidityToken>),
    Array(Vec<SolidityToken>),
    Tuple(Vec<SolidityToken>),
}

impl From<Token> for SolidityToken {
    fn from(token: Token) -> Self {
        match token {
            Token::Address(address) => SolidityToken::Address(format!("{:?}", address)),
            Token::FixedBytes(bytes) => SolidityToken::FixedBytes(bytes),
            Token::Bytes(bytes) => SolidityToken::Bytes(bytes),
            Token::Int(int) => SolidityToken::Int(format!("{}", int)),
            Token::Uint(uint) => SolidityToken::Uint(format!("{}", uint)),
            Token::Bool(boolean) => SolidityToken::Bool(boolean),
            Token::String(string) => SolidityToken::String(string),
            Token::FixedArray(tokens) => {
                SolidityToken::FixedArray(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Array(tokens) => {
                SolidityToken::Array(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Tuple(tokens) => {
                SolidityToken::Tuple(tokens.into_iter().map(SolidityToken::from).collect())
            }
        }
    }
}

impl From<SolidityToken> for Token {
    fn from(token: SolidityToken) -> Self {
        match token {
            SolidityToken::Address(address) => Token::Address(H160::from_str(&address).unwrap()),
            SolidityToken::FixedBytes(bytes) => Token::FixedBytes(bytes),
            SolidityToken::Bytes(bytes) => Token::Bytes(bytes),
            SolidityToken::Int(int) => Token::Int(U256::from_str_radix(&int, 10).unwrap()),
            SolidityToken::Uint(uint) => Token::Uint(U256::from_str_radix(&uint, 10).unwrap()),
            SolidityToken::Bool(boolean) => Token::Bool(boolean),
            SolidityToken::String(string) => Token::String(string),
            SolidityToken::FixedArray(tokens) => {
                Token::FixedArray(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Array(tokens) => {
                Token::Array(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Tuple(tokens) => {
                Token::Tuple(tokens.into_iter().map(Token::from).collect())
            }
        }
    }
}

