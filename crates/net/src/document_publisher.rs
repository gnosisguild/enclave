// TODO: Create DocumentPublisher actor
// 1. Accept EnclaveEvent::CiphernodeSelected and store selected e3_ids in a blume filter
// 1. Accept EnclaveEvent::PublishDocumentRequested
//    Take the payload and convert to NetCommand::PublishDocument
// 1. Accept NetEvent::DocumentPublishedNotification from NetInterface
//    Determine if we are keeping track of the given e3_id based on DocumentMeta
//    and the e3_id blume filter if so then issue a NetCommand::FetchDocument
// 1. Receive the document from NetEvent::FetchDocumentSucceeded and convert to
//    EnclaveEvent::DocumentReceived
// 1. Accept NetEvent::FetchDocumentFailed and attempt to retry
