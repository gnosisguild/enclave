import express, { Request, Response } from 'express';
import { handleRpc } from 'typed-rpc/server';

const app = express();

app.use(express.json());

app.post('/', (req: Request, res: Response) => {
  handleRpc(req.body, {
    // This is called before a computation is attempted. You can use it to prevent unecessary computation.
    shouldCompute(e3Params: string, ciphertextInputs: Array<[string, number]>) {
      return ciphertextInputs.length > 0
    },

    // This is called after computation has occurred
    async processOutput(e3Id: number, proof: string, ciphertext: string) {


      console.log({ e3Id, proof, ciphertext })

      /*
      
      const sdk = new EnclaveSdk(...);
      await sdk.publishCiphertext(e3Id, proof, ciphertext);
      
      */

      return 0;
    },


    // This informs the caller of what methods are available on this server
    capabilities() {
      return [
        "shouldCompute", // optional
        "processOutput" // mandatory
      ]
    }
  }).then(result => res.json(result));
});

app.listen(8080);
