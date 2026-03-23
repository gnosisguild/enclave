import { useState } from "react";
import { closeAuction, AuctionResult as Result } from "../hooks/useAuction";

interface Props {
  auctionId: number;
  numBids: number;
}

export default function AuctionResult({ auctionId, numBids }: Props) {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<Result | null>(null);
  const [error, setError] = useState("");

  const handleClose = async () => {
    setLoading(true);
    setError("");
    try {
      const res = await closeAuction(auctionId);
      setResult(res);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="card">
      <h2>Auction Result</h2>
      {result ? (
        <div className="result">
          <p>
            Winner: <strong>{result.winner_address}</strong>
          </p>
          <p>
            Winning Bid: <strong>{result.winning_bid}</strong>
          </p>
        </div>
      ) : (
        <>
          <p>{numBids} bid(s) submitted. Close the auction to run the homomorphic comparison.</p>
          <button onClick={handleClose} disabled={loading || numBids < 1}>
            {loading
              ? "Running FHE comparison..."
              : "Close Auction & Reveal Winner"}
          </button>
        </>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
}
