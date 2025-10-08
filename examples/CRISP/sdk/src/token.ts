import { CRISP_SERVER_TOKEN_TREE_ENDPOINT } from './constants'

/**
 * Get the merkle tree data from the CRISP server
 * @param serverUrl - The base URL of the CRISP server
 * @param e3Id - The e3Id of the round
 */
export const getTreeData = async (serverUrl: string, e3Id: number) => {
  try {
    const response = await fetch(`${serverUrl}/${CRISP_SERVER_TOKEN_TREE_ENDPOINT}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ "round_id": e3Id }),
    })
  
    console.log('response', response)
  } catch(error) {
    console.error('Error fetching tree data:', error)
  }
}


export const generateMerkleProof = () => {

}

