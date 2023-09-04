const contract = props.contract || "chat-simple.ws-protocol-63";

State.init({
  bootstraping: true,
  loggedIn: false,
  channelList: [],
  selectedChannel: 0,
  usersList: [],
  chatMessages: [],
  message: "",
  inputId: 0,
});

// DATA FETCHING FUNCTIONS - VIEW CALLS
const updateMemberList = () =>
  Near.asyncCalimeroView(contract, "get_members").then((m) => {
    State.update({ usersList: m });
    return m;
  });

const updateChannelList = () =>
  Near.asyncCalimeroView(
    contract,
    "get_groups",
    { account: context.accountId },
    undefined,
    true
  ).then((c) => State.update({ channelList: c }));

const setMessages = () => {
  if (state.channelList[0]) {
    Near.asyncCalimeroView(
      contract,
      "get_messages",
      {
        group: state.channelList[0],
      },
      undefined,
      true
    ).then((m) => {
      State.update({ chatMessages: m });
    });
  }
};

// HELPER FUNCTIONS - CHANGE DATA OR CHANGE FUNCTIONS
const onChangeMessage = ({ target }) => {
  State.update({ message: target.value });
};

const updateInputId = (id) => {
  State.update({ inputId: id });
};
const sendMessage = () => {
  if (!state.message) {
    return;
  }
  let params = {};
  params = { group: state.channelList[0] };
  params.message = state.message;
  params.timestamp = Date.now();
  State.update({ message: "" });
  updateInputId(Math.random().toString(36));
  Near.fakCalimeroCall(contract, "send_message", params);
  setMessages();
};

// CALIMERO FUNCTION ACCESSKEYS FUNCTIONS
const joinCurb = () => {
  Near.requestCalimeroFak(contract);
};

const isMember = (accountId, members) => {
  return (members || state.usersList)
    .map((user) => user.id)
    .includes(accountId);
};

const verifyKey = () => {
  Near.hasValidCalimeroFak(contract).then((result) => {
    State.update({ bootstraping: false, loggedIn: result });
    if (result) {
      updateMemberList().then((members) => {
        if (!isMember(context.accountId, members)) {
          Near.fakCalimeroCall(contract, "join");
        }
      });
      updateChannelList();
    }
  });
};

if (state.bootstraping) {
  verifyKey();
}

updateMemberList();
updateChannelList();
setMessages();

const Separator = styled.div`
  height: 1px;
  padding: 1px;
  width: 3rem;
  background-color: #111;
`;

const Message = styled.div`
  margin-bottom: 1rem;
`;

const formatTimeAgo = (seconds) => {
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const weeks = Math.floor(days / 7);
  const months = Math.floor(weeks / 4);

  if (months > 0) {
    return `${months} month${months > 1 ? "s" : ""} ago`;
  } else if (weeks > 0) {
    return `${weeks} week${weeks > 1 ? "s" : ""} ago`;
  } else if (days > 0) {
    return `${days} day${days > 1 ? "s" : ""} ago`;
  } else if (hours > 0) {
    return `${hours} hour${hours > 1 ? "s" : ""} ago`;
  } else if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? "s" : ""} ago`;
  } else {
    return `just now`;
  }
};

const MessageData = styled.div`
  display: flex;
  column-gap: 1rem;
`;
return (
  <div>
    {context.accountId ? (
      <>
        {state.bootstraping ? (
          <div>Loading...</div>
        ) : (
          <>
            {state.loggedIn && isMember(context.accountId) ? (
              <>
                <div className="d-flex">
                  <div>
                    <div>{state.channelList[0].name.toUpperCase()}</div>
                    {state.chatMessages.length === 0 && <p>No messages yet</p>}
                    <div>
                      {state.chatMessages.map((message, id) => (
                        <Message key={id}>
                          <MessageData>
                            <Widget
                              src="fran-cali.testnet/widget/UserProfileIcon"
                              props={{
                                accountId: props.message.sender,
                                showStatus: false,
                              }}
                            />
                            <p>{message.sender}</p>
                            <p>
                              {formatTimeAgo(
                                (Date.now() - message.timestamp) / 1000
                              )}
                            </p>
                          </MessageData>

                          <p>{message.text}</p>
                          <Separator />
                        </Message>
                      ))}
                    </div>
                    <div className="d-flex gap-x-2">
                      <input
                        onChange={onChangeMessage}
                        onKeyUp={(e) => {
                          if (e.key == "Enter") {
                            sendMessage();
                          }
                        }}
                        placeholder={"send a message"}
                        key={state.inputId}
                        value={state.message}
                        autoFocus
                      />
                      <i
                        className="bi bi-send-fill"
                        onClick={() => sendMessage()}
                      ></i>
                    </div>
                  </div>
                </div>
              </>
            ) : (
              <div onClick={joinCurb}>Join Chat</div>
            )}
          </>
        )}
      </>
    ) : (
      <div>Please login to bos</div>
    )}
  </div>
);
