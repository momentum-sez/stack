const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table,
  spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter11() {
  return [
    pageBreak(),
    chapterHeading("Chapter 11: Smart Asset Virtual Machine"),

    // --- 11.1 Architecture ---
    h2("11.1 Architecture"),
    p("The Smart Asset Virtual Machine (SAVM) is a stack-based execution environment purpose-built for programmable compliance and asset lifecycle management. SAVM comprises four components: the instruction set defining operations across twelve categories, the compliance coprocessor providing direct access to tensor evaluation and ZK verification, the migration engine handling cross-jurisdiction asset transfers with receipt chain integrity, and the gas metering system preventing unbounded computation while pricing compliance operations according to their cryptographic cost."),

    // --- 11.2 Instruction Categories ---
    h2("11.2 Instruction Categories"),
    table(
      ["Range", "Category", "Instructions"],
      [
        ["0x00-0x0F", "Stack", "PUSH, POP, DUP, SWAP"],
        ["0x10-0x1F", "Arithmetic", "ADD, SUB, MUL, DIV, MOD"],
        ["0x20-0x2F", "Comparison", "EQ, LT, GT, AND, OR, NOT"],
        ["0x30-0x3F", "Memory", "MLOAD, MSTORE, MSIZE"],
        ["0x40-0x4F", "Storage", "SLOAD, SSTORE, SDELETE"],
        ["0x50-0x5F", "Control Flow", "JUMP, JUMPI, CALL, RETURN, REVERT"],
        ["0x60-0x6F", "Context", "CALLER, ORIGIN, JURISDICTION, TIMESTAMP"],
        ["0x70-0x7F", "Compliance", "TENSOR_GET, TENSOR_SET, ATTEST, VERIFY_ZK"],
        ["0x80-0x8F", "Migration", "LOCK, UNLOCK, TRANSIT, SETTLE"],
        ["0x90-0x9F", "Crypto", "HASH, VERIFY_SIG, MERKLE_VERIFY"],
        ["0xF0-0xFF", "System", "HALT, LOG, DEBUG"],
      ],
      [1800, 2000, 5560]
    ),
    spacer(),

    // --- 11.3 Compliance Coprocessor ---
    h2("11.3 Compliance Coprocessor"),
    p_runs([
      bold("Tensor Operations. "),
      "The coprocessor provides direct access to the compliance tensor through TENSOR_GET and TENSOR_SET instructions. TENSOR_GET retrieves the compliance state for a given (asset, jurisdiction, domain) triple. TENSOR_SET updates a tensor cell with a new compliance state, requiring an attestation reference and valid authority signature."
    ]),
    p_runs([
      bold("ZK Verification. "),
      "The VERIFY_ZK instruction delegates proof verification to the coprocessor, which selects the appropriate verifier (Plonky3, Groth16, or BBS+) based on the proof type tag. Verification results are pushed onto the stack as boolean values. Invalid proofs consume gas but do not halt execution, allowing contracts to handle verification failure gracefully."
    ]),
    p_runs([
      bold("Migration Support. "),
      "The LOCK, UNLOCK, TRANSIT, and SETTLE instructions implement the cross-jurisdiction migration protocol. LOCK freezes an asset in the source jurisdiction and emits a lock receipt. TRANSIT creates a migration proof binding the lock receipt to the destination jurisdiction. SETTLE finalizes the migration by verifying the transit proof and unlocking the asset in the destination."
    ]),
    ...codeBlock(
      "/// SAVM execution context with compliance coprocessor.\n" +
      "#[derive(Debug)]\n" +
      "pub struct SavmContext {\n" +
      "    pub stack: Vec<U256>,\n" +
      "    pub memory: Vec<u8>,\n" +
      "    pub storage: HashMap<U256, U256>,\n" +
      "    pub pc: usize,\n" +
      "    pub gas_remaining: u64,\n" +
      "    pub caller: EntityId,\n" +
      "    pub origin: EntityId,\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub timestamp: chrono::DateTime<chrono::Utc>,\n" +
      "    pub tensor: ComplianceTensor,\n" +
      "    pub receipt_chain: Vec<ExecutionReceipt>,\n" +
      "}"
    ),
    spacer(),

    // --- 11.4 Gas Metering ---
    h2("11.4 Gas Metering"),
    table(
      ["Category", "Base Gas", "Notes"],
      [
        ["Stack operations", "3", "PUSH, POP, DUP, SWAP"],
        ["Arithmetic", "5", "ADD, SUB, MUL; DIV/MOD cost 8"],
        ["Storage read", "200", "SLOAD; cached reads cost 100"],
        ["Storage write", "5000", "SSTORE; refund 4800 on delete"],
        ["Compliance tensor", "10000", "TENSOR_GET/SET; includes Poseidon2 hash"],
        ["ZK verification", "50000-500000", "Varies by proof system and circuit size"],
        ["Migration", "100000", "LOCK/UNLOCK/TRANSIT/SETTLE; includes receipt generation"],
      ],
      [2400, 2400, 4560]
    ),
    spacer(),

    // --- 11.5 Execution Receipts ---
    h2("11.5 Execution Receipts"),
    p("Every SAVM execution produces a receipt containing the execution digest (SHA-256 of bytecode, input, and context), gas consumed, storage mutations as a set of key-value diffs, compliance tensor updates referencing affected cells and new states, emitted logs, and the execution outcome (success, revert, or out-of-gas). Receipts form an append-only chain, with each receipt referencing the digest of its predecessor. The receipt chain provides a complete audit trail of all programmable asset operations within a jurisdiction."),
  ];
};
