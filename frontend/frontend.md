Act as an elite Web3 and Frontend Engineer. We are building a high-quality, production-ready frontend for a Compute Token Platform where users mint tokens to pay for decentralized compute resources. 

### Tech Stack Constraints:
- Framework: Next.js 14+ (App Router, TypeScript)
- Styling: Tailwind CSS (with clean utility classes, utilizing a modern dark mode theme)
- Icons: lucide-react
- UI Components: Follow a headless/shadcn design style (clean borders, good padding, consistent radius)

### Design Theme & Aesthetic:
- Theme: Dark Mode by default. Deep slate/zinc backgrounds (e.g., `bg-zinc-950`), muted borders (`border-zinc-800`), white text, and a vibrant accent color for primary actions like minting (e.g., emerald-500 or cyan-500).
- Feel: High-end SaaS combined with financial/infrastructure precision. Clean typography, excellent spacing, and subtle glassmorphic effects (`bg-zinc-900/50 backdrop-blur-md`).

### Layout & Component Structure to Generate:
Create a single, highly modular dashboard layout file (or break into clean components if creating a folder structure) containing the following sections:

1. **Navigation Bar (Top):**
   - Left side: Clean platform logo/branding placeholder.
   - Right side: A mock "Connect Wallet" button. If "connected", it should show a truncated address (e.g., `0x7a...2f4c`) with a small green status indicator and the current token balance next to it.

2. **Hero Stat Grid (3 Columns):**
   - Card 1: **Compute Token Balance** (Large bold number, e.g., `14,250 cTMP`, with a sparkline or directional trend up/down).
   - Card 2: **Active Compute Nodes / Resource Usage** (e.g., `3 Active Tasks | 84% Core Load` with a subtle progress bar).
   - Card 3: **Total Spending / Minted History** (e.g., `$420.50 USD Value`).

3. **Main Dashboard Split Section (2 Columns):**
   - **Left Column (60% width) - The Minting Hub:**
     - A beautiful, focused card layout for minting compute tokens.
     - Inputs: A token amount selector (with "Max" button) and an auto-calculated cost estimator showing the conversion rate (e.g., `1 USD = 10 cTMP`).
     - Action: A high-contrast primary button labeled "Mint Compute Tokens". Include a mock disabled loading state placeholder for when a transaction is processing.
   - **Right Column (40% width) - Active Tasks / Live Status:**
     - A clean list showing currently active compute tasks running on the network, each with a micro-badge for status (`Running`, `Completed`, `Queued`).

4. **Transaction Ledger / History (Bottom Full-Width):**
   - A perfectly aligned table showing historical transactions.
   - Columns: Transaction Hash (truncated), Action (Mint / Compute Payment), Amount, Timestamp, and Status (with colored text/icons for Success or Pending).

### Code Quality Rules:
- Write semantic, highly scannable TypeScript interfaces for all mock data shapes.
- Group Tailwind classes logically (Layout -> Spacing -> Sizing -> Borders -> Colors).
- Add clear comments explaining where the state for wallet connection and backend API integration should be wired up later.
- Ensure the layout is fully responsive (stacks vertically on mobile, expands beautifully on desktop).

Generate the complete code for this layout now.