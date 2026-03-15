# Atropos Product Guidelines

## Prose Style
- **Technical & Professional:** Documentation and communication should be clear, concise, and focused on technical accuracy. Avoid marketing fluff.
- **Action-Oriented:** Use active voice and imperative mood for instructions and operational procedures.
- **Context-Rich:** Always explain the "why" behind architectural and operational decisions, referencing ADRs where appropriate.

## Branding & Visual Identity
- **Name:** "Atropos" (the Fate who cuts the thread of life, symbolizing the definitive end of a lease).
- **Theme:** Dark, high-contrast, and industrial. Reflects the robustness and "Best of the Best" status.
- **Dashboard Aesthetic:** Lightweight, functional, and minimal. Use HTMX for interactive elements without heavy client-side frameworks.

## User Experience (UX) Principles
- **Predictability Over Magic:** The system's behavior should be deterministic. Resource allocation and reclamation must follow strict rules without hidden side effects.
- **Fail-Fast & Transparent:** Provide clear, actionable error messages. If an allocation fails, the reason (no capacity, pool locked, etc.) must be explicit.
- **Observability First:** Every user action and system event must be traceable. Metrics and logs are part of the UX.

## Engineering Principles
- **Hexagonal Purity:** Maintain strict isolation between domain logic and infrastructure.
- **No Compromise on Consistency:** Linearizability is non-negotiable. Always use `SKIP LOCKED` and transaction-based allocation.
- **Type-Safe Foundations:** Use the Newtype pattern for IDs to prevent primitive obsession and ensure compile-time safety.
- **Performance-Driven:** Every dependency and feature must be evaluated for its impact on performance and build times.
