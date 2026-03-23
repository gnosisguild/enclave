import { useState } from "react";
import { encryptBid, submitBid } from "../hooks/useAuction";

interface Props {
  auctionId: number;
  onBidSubmitted: () => void;
}

export default function SubmitBid({ auctionId, onBidSubmitted }: Props) {
  const [name, setName] = useState("");
  const [bid, setBid] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !bid.trim()) return;

    const bidValue = parseInt(bid, 10);
    if (isNaN(bidValue) || bidValue < 0 || bidValue >= 1024) {
      setError("Bid must be 0-1023");
      return;
    }

    setLoading(true);
    setError("");
    setStatus("Encrypting bid...");

    try {
      const ciphertext = await encryptBid(auctionId, bidValue);
      setStatus("Submitting encrypted bid...");
      await submitBid(auctionId, name.trim(), ciphertext);
      setStatus(`Bid submitted for ${name.trim()}`);
      setName("");
      setBid("");
      onBidSubmitted();
    } catch (e: any) {
      setError(e.message);
      setStatus("");
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="card">
      <h2>Submit Bid (Auction #{auctionId})</h2>
      <form onSubmit={handleSubmit}>
        <div className="field">
          <label>Bidder Name</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Alice"
            disabled={loading}
          />
        </div>
        <div className="field">
          <label>Bid Amount (0-1023)</label>
          <input
            type="number"
            value={bid}
            onChange={(e) => setBid(e.target.value)}
            placeholder="e.g. 500"
            min={0}
            max={1023}
            disabled={loading}
          />
        </div>
        <button type="submit" disabled={loading || !name.trim() || !bid.trim()}>
          {loading ? "Encrypting..." : "Encrypt & Submit Bid"}
        </button>
      </form>
      {status && <p className="status">{status}</p>}
      {error && <p className="error">{error}</p>}
    </section>
  );
}
