const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, pageBreak, definition
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

    // --- 11.3 Compliance Coprocessor ---
    h3("11.2.1 Compliance Coprocessor"),
    p_runs([
      bold("Tensor Operations. "),
      "The coprocessor provides direct access to the compliance tensor through TENSOR_GET and TENSOR_SET instructions. TENSOR_GET retrieves the compliance state for a given (asset, jurisdiction, domain) triple. TENSOR_SET updates a tensor cell with a new compliance state, requiring an attestation reference and valid authority signature."
    ]),
    p_runs([
      bold("ZK Verification. "),
      "The VERIFY_ZK instruction delegates proof verification to the coprocessor, which selects the appropriate verifier (Groth16 or Plonk) based on the proof type tag. Verification results are pushed onto the stack as boolean values. Invalid proofs consume gas but do not halt execution, allowing contracts to handle verification failure gracefully."
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

    // --- 11.4 Gas Metering ---
    h3("11.2.2 Gas Metering"),
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

    // --- 11.5 SAVM Execution Context ---
    h2("11.3 Execution Context"),
    p("The SAVM execution context encapsulates all state required to execute a compliance-aware smart asset program. The following example traces a complete compliance check execution flow for a cross-border payment from a Pakistan EZ entity to a UAE free zone counterparty."),
    definition("Example 11.1 (Cross-Border Payment Compliance Check).", "An entity in KEZ (Karachi EZ) initiates a USD 250,000 payment to a DMCC (Dubai) counterparty. The SAVM executes the compliance verification bytecode with the following flow:"),
    ...codeBlock(
      "// Step 1: Initialize execution context\n" +
      "// caller = KEZ entity (EntityId), jurisdiction = PAK\n" +
      "// gas_limit = 2,000,000 (sufficient for tensor + ZK ops)\n" +
      "\n" +
      "PUSH entity_id          // Stack: [entity_id]\n" +
      "PUSH jurisdiction_pak   // Stack: [entity_id, PAK]\n" +
      "PUSH domain_aml          // Stack: [entity_id, PAK, Aml]\n" +
      "TENSOR_GET              // Gas: 10,000. Stack: [aml_status]\n" +
      "// aml_status = COMPLIANT (tensor cell PAK x Aml)\n" +
      "\n" +
      "// Step 2: Verify FATF compliance via ZK proof\n" +
      "PUSH fatf_proof_ref     // Stack: [aml_status, fatf_proof]\n" +
      "PUSH verifier_bbs_plus  // Stack: [aml_status, fatf_proof, BBS+]\n" +
      "VERIFY_ZK               // Gas: 75,000. Stack: [aml_status, true]\n" +
      "// BBS+ selective disclosure: proves FATF compliance\n" +
      "// without revealing underlying KYC data\n" +
      "\n" +
      "// Step 3: Check source jurisdiction tax clearance\n" +
      "PUSH entity_id          // Stack: [aml_status, true, entity_id]\n" +
      "PUSH jurisdiction_pak   // Stack: [..., entity_id, PAK]\n" +
      "PUSH domain_tax         // Stack: [..., entity_id, PAK, TAX]\n" +
      "TENSOR_GET              // Gas: 10,000. Stack: [..., tax_status]\n" +
      "// tax_status = COMPLIANT (withholding tax paid)\n" +
      "\n" +
      "// Step 4: Check destination jurisdiction acceptance\n" +
      "PUSH counterparty_id    // Stack: [..., counterparty_id]\n" +
      "PUSH jurisdiction_uae   // Stack: [..., counterparty_id, UAE]\n" +
      "PUSH domain_payments    // Stack: [..., counterparty_id, UAE, PAYMENTS]\n" +
      "TENSOR_GET              // Gas: 10,000. Stack: [..., payments_status]\n" +
      "\n" +
      "// Step 5: Evaluate all conditions\n" +
      "AND                     // Gas: 5. Combine tax + payments\n" +
      "AND                     // Gas: 5. Combine with ZK result\n" +
      "AND                     // Gas: 5. Combine with AML status\n" +
      "// Stack: [true] — all compliance checks passed\n" +
      "\n" +
      "// Step 6: Emit compliance attestation and return\n" +
      "PUSH attest_payload     // Compliance attestation reference\n" +
      "ATTEST                  // Gas: 10,000. Write attestation to tensor\n" +
      "RETURN                  // Total gas consumed: ~115,015"
    ),
    p("The execution consumed 115,015 gas units across four tensor lookups (40,000), one ZK verification (75,000), three boolean operations (15), and one attestation write (10,000). The compliance coprocessor handled all cryptographic operations transparently, and the final ATTEST instruction recorded the compliance result as a new tensor cell update bound to the execution receipt."),

    // --- 11.6 Execution Receipts ---
    h3("11.3.1 Execution Receipts"),
    p("Every SAVM execution produces a receipt containing the execution digest (SHA-256 of bytecode, input, and context), gas consumed, storage mutations as a set of key-value diffs, compliance tensor updates referencing affected cells and new states, emitted logs, and the execution outcome (success, revert, or out-of-gas). Receipts form an append-only chain, with each receipt referencing the digest of its predecessor. The receipt chain provides a complete audit trail of all programmable asset operations within a jurisdiction."),
    ...codeBlock(
      "/// Receipt produced by every SAVM execution.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ExecutionReceipt {\n" +
      "    /// SHA-256 of (bytecode || input || context) — unique execution ID.\n" +
      "    pub execution_digest: Digest,\n" +
      "    /// Digest of the previous receipt in the chain (zero digest for genesis).\n" +
      "    pub prev_receipt_digest: Digest,\n" +
      "    /// Monotonically increasing sequence number within this asset's chain.\n" +
      "    pub sequence: u64,\n" +
      "    /// Identity of the caller that initiated execution.\n" +
      "    pub caller: EntityId,\n" +
      "    /// Jurisdiction under which execution occurred.\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    /// Total gas consumed during execution.\n" +
      "    pub gas_consumed: u64,\n" +
      "    /// Storage mutations: vector of (key, old_value, new_value) triples.\n" +
      "    pub storage_mutations: Vec<(U256, Option<U256>, Option<U256>)>,\n" +
      "    /// Compliance tensor cells updated during execution.\n" +
      "    pub tensor_updates: Vec<TensorCellUpdate>,\n" +
      "    /// Logs emitted via the LOG instruction.\n" +
      "    pub logs: Vec<ExecutionLog>,\n" +
      "    /// Execution outcome.\n" +
      "    pub outcome: ExecutionOutcome,\n" +
      "    /// Timestamp of execution completion.\n" +
      "    pub timestamp: chrono::DateTime<chrono::Utc>,\n" +
      "}\n" +
      "\n" +
      "/// Outcome of a SAVM execution.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum ExecutionOutcome {\n" +
      "    /// Execution completed successfully.\n" +
      "    Success,\n" +
      "    /// Execution reverted with a reason string.\n" +
      "    Revert(String),\n" +
      "    /// Execution ran out of gas at the given program counter.\n" +
      "    OutOfGas { pc: usize, gas_limit: u64 },\n" +
      "}"
    ),
  ];
};
