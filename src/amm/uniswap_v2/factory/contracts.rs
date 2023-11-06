use ethers::prelude::abigen;

abigen!(
    IGetUniswapV2PairsBatchRequest,
        "src/contract/abi/GetUniswapV2PairsBatchRequestABI.json";

    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        function allPairs(uint256 index) external view returns (address)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        function allPairsLength() external view returns (uint256)

    ]"#;
);
