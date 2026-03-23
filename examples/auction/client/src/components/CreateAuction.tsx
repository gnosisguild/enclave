import { useState } from "react";
import { createAuction } from "../hooks/useAuction";

interface Props {
  onCreated: (id: number) => void;
}

export default function CreateAuction({ onCreated }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleCreate = async () => {
    setLoading(true);
    setError("");
    try {
      const { id } = await createAuction();
      onCreated(id);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="card">
      <h2>Create Auction</h2>
      <p>Start a new sealed-bid auction. Bids are encrypted with FHE and compared homomorphically.</p>
      <button onClick={handleCreate} disabled={loading}>
        {loading ? "Creating..." : "Create Auction"}
      </button>
      {error && <p className="error">{error}</p>}
    </section>
  );
}
