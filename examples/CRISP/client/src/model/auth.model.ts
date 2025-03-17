export interface Auth {
  jwt_token: string
  response: 'Already Authorized' | 'No Authorization'
}