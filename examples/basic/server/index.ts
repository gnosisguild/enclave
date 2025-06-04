import express, { Request, Response } from 'express';
import { handleRpc } from 'typed-rpc/server';

const app = express();

app.use(express.json());

app.post('/', (req: Request, res: Response) => {
  handleRpc(req.body, {
    shouldCompute(e3Params: string, ciphertextInputs: Array<[string, number]>) {
      return true;
    },

    processOutput(proof: string, ciphertext: string) {
      console.log({ proof, ciphertext })
      return 0;
    },

    capabilities() {
      return ["shouldCompute", "processOutput"]
    }
  }).then(result => res.json(result));
});

app.listen(8080);
